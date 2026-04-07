//! Repository sync engine — RPM (yum) and APT (deb).

use std::sync::Arc;

use anyhow::{Context, Result};
use native_db::Database;
use uuid::Uuid;

use crate::config::Config;
use crate::db::models::*;
use super::repodata;
use super::errata as errata_parser;
use super::deb;
use super::download::{self, ProgressMap};

/// Build an HTTP client with optional SSL client cert auth (for RHEL CDN).
pub(crate) fn build_client(repo: &Repository) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true);

    if let (Some(cert_path), Some(key_path)) = (&repo.ssl_client_cert, &repo.ssl_client_key) {
        let cert_pem = std::fs::read(cert_path)?;
        let key_pem = std::fs::read(key_path)?;
        let identity = reqwest::Identity::from_pem(&[cert_pem, key_pem].concat())?;
        builder = builder.identity(identity);
    }

    builder.build().map_err(Into::into)
}

/// Apply HTTP Basic Auth if repo has username/password.
pub(crate) fn apply_auth(req: reqwest::RequestBuilder, repo: &Repository) -> reqwest::RequestBuilder {
    if let (Some(user), Some(pass)) = (&repo.username, &repo.password) {
        req.basic_auth(user, Some(pass))
    } else {
        req
    }
}

/// Sync a repository: fetch metadata, parse packages and errata, store in DB, then download packages.
pub async fn sync_repo(
    db: &Arc<Database<'static>>,
    repo_id: &str,
    config: &Arc<Config>,
    progress: &ProgressMap,
) -> Result<()> {
    // Load repo from DB
    let repo = {
        let r = db.r_transaction()?;
        r.get().primary::<Repository>(repo_id.to_string())?
            .ok_or_else(|| anyhow::anyhow!("repository not found: {}", repo_id))?
    };

    // Skip disabled repos
    if !repo.enabled {
        tracing::info!("Skipping disabled repository '{}'", repo.name);
        return Ok(());
    }

    tracing::info!("Syncing repository '{}' from {}", repo.name, repo.url);

    // Write "started" sync log
    let sync_log = SyncLog::new_started(&repo.id, &repo.name);
    let log_id = sync_log.id.clone();
    {
        let rw = db.rw_transaction()?;
        rw.insert(sync_log).map_err(|e| anyhow::anyhow!("{}", e))?;
        rw.commit()?;
    }

    // Mark as syncing
    {
        let rw = db.rw_transaction()?;
        let old = rw.get().primary::<Repository>(repo_id.to_string())?
            .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
        let mut updated = old.clone();
        updated.sync_state = RepoSyncState::Syncing;
        rw.update(old, updated)?;
        rw.commit()?;
    }

    let result = match repo.content_type.as_str() {
        "deb" => do_sync_deb(db, &repo).await,
        _ => do_sync_yum(db, &repo).await,
    };

    // Update final state
    let rw = db.rw_transaction()?;
    let old = rw.get().primary::<Repository>(repo_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let mut updated = old.clone();

    match &result {
        Ok((pkg_count, errata_count)) => {
            // Compute total_size_bytes from package metadata
            let total_size: u64 = {
                let r2 = db.r_transaction()?;
                let all_pkgs: Vec<Package> = r2.scan().primary()
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                    .all()
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                all_pkgs.iter().filter(|p| p.repo_id == repo.id).map(|p| p.size).sum()
            };

            updated.package_count = *pkg_count;
            updated.errata_count = *errata_count;
            updated.total_size_bytes = total_size;
            updated.last_sync = Some(chrono::Utc::now().to_rfc3339());

            // Update repo before download phase so total_size is visible
            rw.update(old, updated.clone())?;
            rw.commit()?;

            // Initialize progress for metadata phase
            {
                let mut map = progress.lock().await;
                let entry = map.entry(repo.id.clone()).or_insert_with(download::SyncProgress::new);
                entry.phase = "metadata_complete".to_string();
                entry.total_packages = *pkg_count;
                entry.total_size_bytes = total_size;
            }

            // Download packages to disk
            tracing::info!("Starting package downloads: {} packages, {} total size",
                pkg_count, download::format_bytes(total_size));

            let (dl, sk, fl, bytes) = download::download_all_packages(
                db, &updated, config, progress,
            ).await?;

            // Update repo with download stats
            let rw2 = db.rw_transaction()?;
            let old2 = rw2.get().primary::<Repository>(repo_id.to_string())?
                .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
            let mut final_repo = old2.clone();
            final_repo.sync_state = RepoSyncState::Synced;
            final_repo.downloaded_size_bytes = bytes + {
                // Add previously downloaded bytes from skipped packages
                let r3 = db.r_transaction()?;
                let all: Vec<Package> = r3.scan().primary()
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                    .all()
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                all.iter()
                    .filter(|p| p.repo_id == repo.id && p.downloaded)
                    .map(|p| p.download_size)
                    .sum::<u64>()
            };
            final_repo.downloaded_package_count = dl + sk;
            rw2.update(old2, final_repo)?;

            tracing::info!("Sync complete: {} packages, {} errata, {} downloaded, {} skipped, {} failed",
                pkg_count, errata_count, dl, sk, fl);

            // Update sync log to success
            if let Ok(Some(old_log)) = rw2.get().primary::<SyncLog>(log_id.clone()) {
                let mut log = old_log.clone();
                log.status = "success".to_string();
                log.message = format!("{} packages, {} errata, {} downloaded, {} skipped",
                    pkg_count, errata_count, dl, sk);
                log.packages_synced = *pkg_count;
                log.errata_synced = *errata_count;
                log.packages_downloaded = dl;
                log.packages_skipped = sk;
                log.bytes_downloaded = bytes;
                log.total_size_bytes = total_size;
                log.finished_at = Some(chrono::Utc::now().to_rfc3339());
                let _ = rw2.update(old_log, log);
            }

            rw2.commit()?;

            // Clean up progress entry
            {
                let mut map = progress.lock().await;
                map.remove(&repo.id);
            }
        }
        Err(e) => {
            updated.sync_state = RepoSyncState::Failed;
            tracing::error!("Sync failed for '{}': {}", repo.name, e);

            // Update sync log to failed
            if let Ok(Some(old_log)) = rw.get().primary::<SyncLog>(log_id.clone()) {
                let mut log = old_log.clone();
                log.status = "failed".to_string();
                log.message = e.to_string();
                log.finished_at = Some(chrono::Utc::now().to_rfc3339());
                let _ = rw.update(old_log, log);
            }

            rw.update(old, updated)?;
            rw.commit()?;

            // Clean up progress entry
            {
                let mut map = progress.lock().await;
                map.remove(&repo.id);
            }
        }
    }

    result.map(|_| ())
}

async fn do_sync_yum(db: &Arc<Database<'static>>, repo: &Repository) -> Result<(u64, u64)> {
    let client = build_client(repo)?;

    let base_url = repo.url.trim_end_matches('/');

    // 1. Fetch repomd.xml
    let repomd_url = format!("{}/repodata/repomd.xml", base_url);
    tracing::info!("Fetching {}", repomd_url);
    let repomd_xml = apply_auth(client.get(&repomd_url), repo).send().await?
        .error_for_status()
        .context("failed to fetch repomd.xml")?
        .text().await?;

    let entries = repodata::parse_repomd(&repomd_xml)?;

    // 2. Find and fetch primary.xml.gz
    let primary_entry = entries.iter()
        .find(|e| e.data_type == "primary")
        .ok_or_else(|| anyhow::anyhow!("no primary entry in repomd.xml"))?;

    let primary_url = format!("{}/{}", base_url, primary_entry.location);
    tracing::info!("Fetching {}", primary_url);
    let primary_gz = apply_auth(client.get(&primary_url), repo).send().await?
        .error_for_status()
        .context("failed to fetch primary.xml.gz")?
        .bytes().await?;

    let primary_xml = repodata::decompress_gz(&primary_gz)?;
    let parsed_packages = repodata::parse_primary(&primary_xml)?;

    // 3. Store packages in DB (clear old ones first, then insert new)
    let rw = db.rw_transaction()?;

    // Remove existing packages for this repo
    let existing: Vec<Package> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for pkg in existing.iter().filter(|p| p.repo_id == repo.id) {
        rw.remove(pkg.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    // Insert new packages
    let pkg_count = parsed_packages.len() as u64;
    for pp in &parsed_packages {
        let pkg = Package {
            id: Uuid::new_v4().to_string(),
            repo_id: repo.id.clone(),
            name: pp.name.clone(),
            epoch: pp.epoch.clone(),
            version: pp.version.clone(),
            release: pp.release.clone(),
            arch: pp.arch.clone(),
            summary: pp.summary.clone(),
            sha256: pp.sha256.clone(),
            size: pp.size,
            location_href: pp.location_href.clone(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        rw.insert(pkg).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    // 4. Fetch and parse updateinfo.xml.gz (if available)
    let mut errata_count: u64 = 0;
    if let Some(updateinfo_entry) = entries.iter().find(|e| e.data_type == "updateinfo") {
        let updateinfo_url = format!("{}/{}", base_url, updateinfo_entry.location);
        tracing::info!("Fetching {}", updateinfo_url);

        match apply_auth(client.get(&updateinfo_url), repo).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let updateinfo_gz = resp.bytes().await?;
                    let updateinfo_xml = repodata::decompress_gz(&updateinfo_gz)?;
                    let parsed_errata = errata_parser::parse_updateinfo(&updateinfo_xml)?;

                    let rw = db.rw_transaction()?;

                    // Remove existing errata for this repo
                    let existing: Vec<Erratum> = rw.scan().primary()
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                        .all()
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                    for er in existing.iter().filter(|e| e.repo_id == repo.id) {
                        rw.remove(er.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
                    }

                    errata_count = parsed_errata.len() as u64;
                    for pe in &parsed_errata {
                        let erratum = Erratum {
                            id: Uuid::new_v4().to_string(),
                            advisory_id: pe.advisory_id.clone(),
                            repo_id: repo.id.clone(),
                            title: pe.title.clone(),
                            erratum_type: match pe.erratum_type.as_str() {
                                "Security" => ErratumType::Security,
                                "Enhancement" => ErratumType::Enhancement,
                                _ => ErratumType::Bugfix,
                            },
                            severity: match pe.severity.as_str() {
                                "Critical" => ErratumSeverity::Critical,
                                "Important" => ErratumSeverity::Important,
                                "Moderate" => ErratumSeverity::Moderate,
                                "Low" => ErratumSeverity::Low,
                                _ => ErratumSeverity::None,
                            },
                            description: pe.description.clone(),
                            issued: pe.issued.clone(),
                            updated: pe.updated.clone(),
                            cves: pe.cves.clone(),
                            package_names: pe.package_names.clone(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                        };
                        rw.insert(erratum).map_err(|e| anyhow::anyhow!("{}", e))?;
                    }

                    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;
                }
            }
            Err(e) => {
                tracing::warn!("Could not fetch updateinfo: {}", e);
            }
        }
    }

    Ok((pkg_count, errata_count))
}

/// Sync a deb (APT) repository: fetch Release, then Packages.gz for each component/arch combo.
async fn do_sync_deb(db: &Arc<Database<'static>>, repo: &Repository) -> Result<(u64, u64)> {
    let client = build_client(repo)?;

    let base_url = repo.url.trim_end_matches('/');
    let codename = repo.codename.as_deref().unwrap_or("stable");
    let components: Vec<&str> = repo.components.as_deref().unwrap_or("main")
        .split(',')
        .map(|s| s.trim())
        .collect();
    let architectures: Vec<&str> = repo.architectures.as_deref().unwrap_or("amd64")
        .split(',')
        .map(|s| s.trim())
        .collect();

    // 1. Fetch Release file
    let release_url = format!("{}/dists/{}/Release", base_url, codename);
    tracing::info!("Fetching {}", release_url);
    let _release_text = apply_auth(client.get(&release_url), repo).send().await?
        .error_for_status()
        .context("failed to fetch Release")?
        .text().await?;

    // 2. Fetch Packages.gz for each component/arch combo
    let mut all_deb_packages = Vec::new();

    for component in &components {
        for arch in &architectures {
            let packages_url = format!(
                "{}/dists/{}/{}/binary-{}/Packages.gz",
                base_url, codename, component, arch
            );
            tracing::info!("Fetching {}", packages_url);

            match apply_auth(client.get(&packages_url), repo).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let gz_data = resp.bytes().await?;
                        let text = deb::decompress_packages_gz(&gz_data)?;
                        let pkgs = deb::parse_packages(&text);
                        tracing::info!("Parsed {} packages from {}/{}", pkgs.len(), component, arch);
                        all_deb_packages.extend(pkgs);
                    } else {
                        tracing::warn!("Packages.gz not found for {}/binary-{}: {}", component, arch, resp.status());
                    }
                }
                Err(e) => {
                    tracing::warn!("Could not fetch Packages.gz for {}/binary-{}: {}", component, arch, e);
                }
            }
        }
    }

    // 3. Store packages in DB (clear old, insert new)
    let rw = db.rw_transaction()?;

    let existing: Vec<Package> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for pkg in existing.iter().filter(|p| p.repo_id == repo.id) {
        rw.remove(pkg.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    let pkg_count = all_deb_packages.len() as u64;
    for dp in &all_deb_packages {
        let (epoch, _upstream, revision) = deb::parse_deb_version(&dp.version);
        let pkg = Package {
            id: Uuid::new_v4().to_string(),
            repo_id: repo.id.clone(),
            name: dp.package.clone(),
            epoch,
            version: dp.version.clone(),
            release: revision,
            arch: dp.architecture.clone(),
            summary: dp.description.clone(),
            sha256: dp.sha256.clone(),
            size: dp.size,
            location_href: dp.filename.clone(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        rw.insert(pkg).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    // No errata sync for deb repos
    Ok((pkg_count, 0))
}
