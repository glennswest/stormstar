//! RPM repository sync engine.

use std::sync::Arc;

use anyhow::{Context, Result};
use native_db::Database;
use uuid::Uuid;

use crate::db::models::*;
use super::repodata;
use super::errata as errata_parser;

/// Sync a repository: fetch metadata, parse packages and errata, store in DB.
pub async fn sync_repo(db: &Arc<Database<'static>>, repo_id: &str) -> Result<()> {
    // Load repo from DB
    let repo = {
        let r = db.r_transaction()?;
        r.get().primary::<Repository>(repo_id.to_string())?
            .ok_or_else(|| anyhow::anyhow!("repository not found: {}", repo_id))?
    };

    tracing::info!("Syncing repository '{}' from {}", repo.name, repo.url);

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

    let result = do_sync(db, &repo).await;

    // Update final state
    let rw = db.rw_transaction()?;
    let old = rw.get().primary::<Repository>(repo_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let mut updated = old.clone();

    match &result {
        Ok((pkg_count, errata_count)) => {
            updated.sync_state = RepoSyncState::Synced;
            updated.package_count = *pkg_count;
            updated.errata_count = *errata_count;
            updated.last_sync = Some(chrono::Utc::now().to_rfc3339());
            tracing::info!("Sync complete: {} packages, {} errata", pkg_count, errata_count);
        }
        Err(e) => {
            updated.sync_state = RepoSyncState::Failed;
            tracing::error!("Sync failed for '{}': {}", repo.name, e);
        }
    }

    rw.update(old, updated)?;
    rw.commit()?;

    result.map(|_| ())
}

async fn do_sync(db: &Arc<Database<'static>>, repo: &Repository) -> Result<(u64, u64)> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let base_url = repo.url.trim_end_matches('/');

    // 1. Fetch repomd.xml
    let repomd_url = format!("{}/repodata/repomd.xml", base_url);
    tracing::info!("Fetching {}", repomd_url);
    let repomd_xml = client.get(&repomd_url).send().await?
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
    let primary_gz = client.get(&primary_url).send().await?
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

        match client.get(&updateinfo_url).send().await {
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
