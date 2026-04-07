//! Package download engine — download actual RPM/deb files to local disk.
//!
//! Packages are stored under `{data_dir}/repos/{repo_id}/` with layout:
//!   RPM:  Packages/{first_letter}/{name}-{ver}-{rel}.{arch}.rpm
//!   Deb:  {location_href} (e.g. pool/main/c/curl/curl_7.88-1_amd64.deb)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use native_db::Database;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Semaphore};

use crate::config::Config;
use crate::db::models::*;

/// Live sync progress for a repository.
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub phase: String,
    pub total_packages: u64,
    pub downloaded: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes_downloaded: u64,
    pub total_size_bytes: u64,
    pub current_package: String,
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncProgress {
    pub fn new() -> Self {
        Self {
            phase: "initializing".to_string(),
            total_packages: 0,
            downloaded: 0,
            skipped: 0,
            failed: 0,
            bytes_downloaded: 0,
            total_size_bytes: 0,
            current_package: String::new(),
        }
    }
}

/// Shared progress map: repo_id → live progress.
pub type ProgressMap = Arc<Mutex<HashMap<String, SyncProgress>>>;

/// Create a new empty progress map.
pub fn new_progress_map() -> ProgressMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Compute the local storage path for an RPM package.
pub fn rpm_local_path(pkg: &Package) -> String {
    let first = pkg.name.chars().next()
        .map(|c| c.to_lowercase().to_string())
        .unwrap_or_else(|| "0".to_string());
    format!("Packages/{}/{}", first, pkg.filename())
}

/// Compute the local storage path for a deb package.
/// Uses the upstream location_href which already has the pool layout.
pub fn deb_local_path(pkg: &Package) -> String {
    if pkg.location_href.is_empty() {
        let prefix = pkg.deb_pool_prefix();
        format!("pool/main/{}/{}/{}", prefix, pkg.name, pkg.deb_filename())
    } else {
        pkg.location_href.clone()
    }
}

/// Check if a package is already downloaded (size-match check).
pub fn is_already_downloaded(repo_dir: &Path, local_path: &str, expected_size: u64) -> bool {
    let full = repo_dir.join(local_path);
    match std::fs::metadata(&full) {
        Ok(meta) => meta.len() == expected_size && expected_size > 0,
        Err(_) => false,
    }
}

/// Download a single package file with SHA256 verification.
/// Writes to a .tmp file first, then atomically renames.
async fn download_package(
    client: &reqwest::Client,
    base_url: &str,
    upstream_path: &str,
    dest: &Path,
    expected_sha256: &str,
    repo: &Repository,
) -> Result<u64> {
    let pkg_url = format!("{}/{}", base_url, upstream_path);

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await
            .context("failed to create package directory")?;
    }

    let tmp_path = dest.with_extension("tmp");

    // Build request with auth
    let req = client.get(&pkg_url);
    let req = super::repo::apply_auth(req, repo);

    let resp = req.send().await
        .context("download request failed")?
        .error_for_status()
        .context("upstream returned error")?;

    let bytes = resp.bytes().await
        .context("failed to read response body")?;

    let size = bytes.len() as u64;

    // SHA256 verification (skip if expected is empty)
    if !expected_sha256.is_empty() {
        let actual = hex::encode(Sha256::digest(&bytes));
        if actual != expected_sha256 {
            anyhow::bail!(
                "SHA256 mismatch: expected {}, got {} for {}",
                expected_sha256, actual, upstream_path
            );
        }
    }

    // Write to temp file
    tokio::fs::write(&tmp_path, &bytes).await
        .context("failed to write temp file")?;

    // Atomic rename
    tokio::fs::rename(&tmp_path, dest).await
        .context("failed to rename temp file")?;

    Ok(size)
}

/// Download all packages for a repository concurrently.
///
/// Returns (downloaded, skipped, failed, bytes_downloaded).
pub async fn download_all_packages(
    db: &Arc<Database<'static>>,
    repo: &Repository,
    config: &Arc<Config>,
    progress: &ProgressMap,
) -> Result<(u64, u64, u64, u64)> {
    let repo_dir = PathBuf::from(&config.data_dir)
        .join("repos")
        .join(&repo.id);

    // Ensure repo directory exists
    tokio::fs::create_dir_all(&repo_dir).await?;

    // Load all packages for this repo
    let packages: Vec<Package> = {
        let r = db.r_transaction()?;
        let all: Vec<Package> = r.scan().primary()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .all()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        all.into_iter().filter(|p| p.repo_id == repo.id).collect()
    };

    let total = packages.len() as u64;
    let total_size: u64 = packages.iter().map(|p| p.size).sum();

    // Initialize progress
    {
        let mut map = progress.lock().await;
        let entry = map.entry(repo.id.clone()).or_insert_with(SyncProgress::new);
        entry.phase = "downloading".to_string();
        entry.total_packages = total;
        entry.total_size_bytes = total_size;
    }

    let client = super::repo::build_client(repo)?;
    let base_url = repo.url.trim_end_matches('/').to_string();
    let semaphore = Arc::new(Semaphore::new(config.download_concurrency));

    let downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let skipped = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let failed = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let bytes_total = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let mut handles = Vec::new();

    for pkg in packages {
        let local_path = if repo.content_type == "deb" {
            deb_local_path(&pkg)
        } else {
            rpm_local_path(&pkg)
        };

        // Check if already downloaded
        if is_already_downloaded(&repo_dir, &local_path, pkg.size) {
            skipped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Update package record if not already marked
            if !pkg.downloaded {
                let db2 = db.clone();
                let lp = local_path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(rw) = db2.rw_transaction() {
                        if let Ok(Some(old)) = rw.get().primary::<Package>(pkg.id.clone()) {
                            let mut updated = old.clone();
                            updated.downloaded = true;
                            updated.local_path = lp;
                            updated.download_size = pkg.size;
                            let _ = rw.update(old, updated);
                            let _ = rw.commit();
                        }
                    }
                }).await;
            }

            let progress2 = progress.clone();
            let repo_id = repo.id.clone();
            let mut map = progress2.lock().await;
            if let Some(entry) = map.get_mut(&repo_id) {
                entry.skipped += 1;
            }

            continue;
        }

        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let base_url = base_url.clone();
        let repo_dir = repo_dir.clone();
        let repo_clone = repo.clone();
        let db2 = db.clone();
        let downloaded2 = downloaded.clone();
        let failed2 = failed.clone();
        let bytes_total2 = bytes_total.clone();
        let progress2 = progress.clone();
        let repo_id = repo.id.clone();
        let pkg_name = pkg.name.clone();

        let handle = tokio::spawn(async move {
            // Update current package in progress
            {
                let mut map = progress2.lock().await;
                if let Some(entry) = map.get_mut(&repo_id) {
                    entry.current_package = pkg_name;
                }
            }

            let upstream_path = pkg.location_href.clone();

            let dest = repo_dir.join(&local_path);

            match download_package(
                &client,
                &base_url,
                &upstream_path,
                &dest,
                &pkg.sha256,
                &repo_clone,
            ).await {
                Ok(size) => {
                    downloaded2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    bytes_total2.fetch_add(size, std::sync::atomic::Ordering::Relaxed);

                    // Update package record in DB
                    let lp = local_path.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(rw) = db2.rw_transaction() {
                            if let Ok(Some(old)) = rw.get().primary::<Package>(pkg.id.clone()) {
                                let mut updated = old.clone();
                                updated.downloaded = true;
                                updated.local_path = lp;
                                updated.download_size = size;
                                let _ = rw.update(old, updated);
                                let _ = rw.commit();
                            }
                        }
                    }).await;

                    // Update progress
                    let mut map = progress2.lock().await;
                    if let Some(entry) = map.get_mut(&repo_id) {
                        entry.downloaded += 1;
                        entry.bytes_downloaded += size;
                    }
                }
                Err(e) => {
                    failed2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    tracing::warn!("Failed to download {}: {}", upstream_path, e);
                }
            }

            drop(permit);
        });

        handles.push(handle);
    }

    // Wait for all downloads
    for handle in handles {
        let _ = handle.await;
    }

    let dl = downloaded.load(std::sync::atomic::Ordering::Relaxed);
    let sk = skipped.load(std::sync::atomic::Ordering::Relaxed);
    let fl = failed.load(std::sync::atomic::Ordering::Relaxed);
    let bt = bytes_total.load(std::sync::atomic::Ordering::Relaxed);

    // Finalize progress
    {
        let mut map = progress.lock().await;
        if let Some(entry) = map.get_mut(&repo.id) {
            entry.phase = "complete".to_string();
            entry.current_package.clear();
        }
    }

    tracing::info!(
        "Download complete: {} downloaded, {} skipped, {} failed, {} bytes",
        dl, sk, fl, bt
    );

    Ok((dl, sk, fl, bt))
}

/// Format bytes into a human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    const TIB: u64 = 1024 * 1024 * 1024 * 1024;

    if bytes >= TIB {
        format!("{:.1} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpm_local_path() {
        let pkg = Package {
            id: "1".to_string(),
            repo_id: "r1".to_string(),
            name: "bash".to_string(),
            epoch: "0".to_string(),
            version: "5.1.8".to_string(),
            release: "6.el9".to_string(),
            arch: "x86_64".to_string(),
            summary: String::new(),
            sha256: String::new(),
            size: 1000,
            location_href: "Packages/b/bash-5.1.8-6.el9.x86_64.rpm".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: String::new(),
        };
        assert_eq!(rpm_local_path(&pkg), "Packages/b/bash-5.1.8-6.el9.x86_64.rpm");
    }

    #[test]
    fn test_deb_local_path_with_href() {
        let pkg = Package {
            id: "1".to_string(),
            repo_id: "r1".to_string(),
            name: "curl".to_string(),
            epoch: "0".to_string(),
            version: "7.88.1-10+deb12u8".to_string(),
            release: String::new(),
            arch: "amd64".to_string(),
            summary: String::new(),
            sha256: String::new(),
            size: 500,
            location_href: "pool/main/c/curl/curl_7.88.1-10+deb12u8_amd64.deb".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: String::new(),
        };
        assert_eq!(deb_local_path(&pkg), "pool/main/c/curl/curl_7.88.1-10+deb12u8_amd64.deb");
    }

    #[test]
    fn test_deb_local_path_empty_href() {
        let pkg = Package {
            id: "1".to_string(),
            repo_id: "r1".to_string(),
            name: "libssl3".to_string(),
            epoch: "0".to_string(),
            version: "3.0.0".to_string(),
            release: "1".to_string(),
            arch: "amd64".to_string(),
            summary: String::new(),
            sha256: String::new(),
            size: 500,
            location_href: String::new(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: String::new(),
        };
        assert_eq!(deb_local_path(&pkg), "pool/main/libs/libssl3/libssl3_3.0.0_amd64.deb");
    }

    #[test]
    fn test_is_already_downloaded_missing() {
        let dir = std::path::Path::new("/nonexistent");
        assert!(!is_already_downloaded(dir, "foo.rpm", 100));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1048576), "1.0 MiB");
        assert_eq!(format_bytes(1073741824), "1.0 GiB");
        assert_eq!(format_bytes(1099511627776), "1.0 TiB");
    }
}
