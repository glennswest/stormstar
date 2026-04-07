//! HTTP yum and APT compatible repo serving.
//!
//! YUM repos served at:
//!   /pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/
//!     repodata/repomd.xml, repodata/primary.xml.gz, Packages/<letter>/<file>.rpm
//!
//! APT repos served at:
//!   /pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/
//!     dists/<codename>/Release, dists/<codename>/<comp>/binary-<arch>/Packages[.gz]
//!     pool/<comp>/<prefix>/<source>/<file>.deb

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    body::Body,
};
use native_db::Database;
use tokio_util::io::ReaderStream;

use crate::config::Config;
use crate::content::download::ProgressMap;
use crate::db::models::*;

#[derive(Clone)]
pub struct ContentState {
    pub db: Arc<Database<'static>>,
    pub config: Arc<Config>,
    pub progress: ProgressMap,
}

pub fn routes() -> Router<ContentState> {
    Router::new()
        // YUM routes
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/repodata/repomd.xml",
            get(serve_repomd))
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/repodata/primary.xml.gz",
            get(serve_primary))
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/Packages/{letter}/{filename}",
            get(serve_package))
        // APT routes
        .route("/pulp/deb/{org}/{env}/{cv}/custom/{product}/{repo}/dists/{codename}/Release",
            get(serve_deb_release))
        .route("/pulp/deb/{org}/{env}/{cv}/custom/{product}/{repo}/dists/{codename}/InRelease",
            get(serve_deb_release))
        .route("/pulp/deb/{org}/{env}/{cv}/custom/{product}/{repo}/dists/{codename}/{component}/binary-{arch}/Packages",
            get(serve_deb_packages))
        .route("/pulp/deb/{org}/{env}/{cv}/custom/{product}/{repo}/dists/{codename}/{component}/binary-{arch}/Packages.gz",
            get(serve_deb_packages_gz))
        .route("/pulp/deb/{org}/{env}/{cv}/custom/{product}/{repo}/pool/{component}/{prefix}/{source}/{filename}",
            get(serve_deb_pool))
}

/// Resolve a repo by label path components.
fn resolve_repo(
    db: &Database<'static>,
    _org_label: &str,
    _product_label: &str,
    repo_label: &str,
) -> Result<Repository, (StatusCode, String)> {
    let r = db.r_transaction()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let repos: Vec<Repository> = r.scan().primary()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .all()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    repos.into_iter()
        .find(|r| r.label == repo_label)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "repository not found".to_string()))
}

/// Get all packages for a repo.
fn get_repo_packages(db: &Database<'static>, repo_id: &str) -> Result<Vec<Package>, (StatusCode, String)> {
    let r = db.r_transaction()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let all: Vec<Package> = r.scan().primary()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .all()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(all.into_iter().filter(|p| p.repo_id == repo_id).collect())
}

#[derive(serde::Deserialize)]
struct RepoPath {
    org: String,
    #[allow(dead_code)]
    env: String,
    #[allow(dead_code)]
    cv: String,
    product: String,
    repo: String,
}

async fn serve_repomd(
    State(state): State<ContentState>,
    Path(path): Path<RepoPath>,
) -> Result<Response, (StatusCode, String)> {
    let repo = resolve_repo(&state.db, &path.org, &path.product, &path.repo)?;
    let packages = get_repo_packages(&state.db, &repo.id)?;

    let xml = super::repodata::generate_repomd(&packages);

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/xml")
        .body(Body::from(xml))
        .unwrap())
}

async fn serve_primary(
    State(state): State<ContentState>,
    Path(path): Path<RepoPath>,
) -> Result<Response, (StatusCode, String)> {
    let repo = resolve_repo(&state.db, &path.org, &path.product, &path.repo)?;
    let packages = get_repo_packages(&state.db, &repo.id)?;

    let xml = super::repodata::generate_primary(&packages);

    // Gzip compress
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let compressed = encoder.finish()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/gzip")
        .body(Body::from(compressed))
        .unwrap())
}

#[derive(serde::Deserialize)]
struct PackagePath {
    org: String,
    #[allow(dead_code)]
    env: String,
    #[allow(dead_code)]
    cv: String,
    product: String,
    repo: String,
    #[allow(dead_code)]
    letter: String,
    filename: String,
}

async fn serve_package(
    State(state): State<ContentState>,
    Path(path): Path<PackagePath>,
) -> impl IntoResponse {
    let repo = match resolve_repo(&state.db, &path.org, &path.product, &path.repo) {
        Ok(r) => r,
        Err(e) => return Err(e),
    };

    let packages = match get_repo_packages(&state.db, &repo.id) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    // Find the package by filename
    let pkg = packages.iter()
        .find(|p| p.filename() == path.filename)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "package not found".to_string()))?;

    // Try serving from local disk first
    if pkg.downloaded && !pkg.local_path.is_empty() {
        let local_file = std::path::PathBuf::from(&state.config.data_dir)
            .join("repos")
            .join(&repo.id)
            .join(&pkg.local_path);

        if let Ok(file) = tokio::fs::File::open(&local_file).await {
            let meta = file.metadata().await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            return Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/x-rpm")
                .header("Content-Length", meta.len().to_string())
                .body(body)
                .unwrap());
        }
    }

    // Fallback: proxy from upstream
    let base_url = repo.url.trim_end_matches('/');
    let pkg_url = format!("{}/{}", base_url, pkg.location_href);

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resp = client.get(&pkg_url).send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream fetch failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err((StatusCode::BAD_GATEWAY, format!("upstream returned {}", resp.status())));
    }

    let bytes = resp.bytes().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream read failed: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-rpm")
        .header("Content-Length", bytes.len().to_string())
        .body(Body::from(bytes))
        .unwrap())
}

// ── APT (deb) serving handlers ──────────────────────────────────────

#[derive(serde::Deserialize)]
struct DebReleasePath {
    org: String,
    #[allow(dead_code)]
    env: String,
    #[allow(dead_code)]
    cv: String,
    product: String,
    repo: String,
    #[allow(dead_code)]
    codename: String,
}

async fn serve_deb_release(
    State(state): State<ContentState>,
    Path(path): Path<DebReleasePath>,
) -> Result<Response, (StatusCode, String)> {
    let repo = resolve_repo(&state.db, &path.org, &path.product, &path.repo)?;
    let packages = get_repo_packages(&state.db, &repo.id)?;

    let codename = repo.codename.as_deref().unwrap_or("stable");
    let components: Vec<String> = repo.components.as_deref().unwrap_or("main")
        .split(',').map(|s| s.trim().to_string()).collect();
    let architectures: Vec<String> = repo.architectures.as_deref().unwrap_or("amd64")
        .split(',').map(|s| s.trim().to_string()).collect();

    // Build entries for each component/arch Packages file
    let mut entries = Vec::new();
    for component in &components {
        for arch in &architectures {
            let arch_packages: Vec<&Package> = packages.iter()
                .filter(|p| p.arch == *arch || p.arch == "all")
                .collect();
            let packages_text = super::deb::generate_packages(
                &arch_packages.iter().map(|p| (*p).clone()).collect::<Vec<_>>(),
                component,
            );
            let rel_path = format!("{}/binary-{}/Packages", component, arch);
            entries.push((rel_path, packages_text.into_bytes()));
        }
    }

    let release = super::deb::generate_release(codename, &architectures, &components, &entries);

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(Body::from(release))
        .unwrap())
}

#[derive(serde::Deserialize)]
struct DebPackagesPath {
    org: String,
    #[allow(dead_code)]
    env: String,
    #[allow(dead_code)]
    cv: String,
    product: String,
    repo: String,
    #[allow(dead_code)]
    codename: String,
    component: String,
    arch: String,
}

async fn serve_deb_packages(
    State(state): State<ContentState>,
    Path(path): Path<DebPackagesPath>,
) -> Result<Response, (StatusCode, String)> {
    let repo = resolve_repo(&state.db, &path.org, &path.product, &path.repo)?;
    let packages = get_repo_packages(&state.db, &repo.id)?;

    let arch_packages: Vec<Package> = packages.into_iter()
        .filter(|p| p.arch == path.arch || p.arch == "all")
        .collect();

    let text = super::deb::generate_packages(&arch_packages, &path.component);

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(Body::from(text))
        .unwrap())
}

async fn serve_deb_packages_gz(
    State(state): State<ContentState>,
    Path(path): Path<DebPackagesPath>,
) -> Result<Response, (StatusCode, String)> {
    let repo = resolve_repo(&state.db, &path.org, &path.product, &path.repo)?;
    let packages = get_repo_packages(&state.db, &repo.id)?;

    let arch_packages: Vec<Package> = packages.into_iter()
        .filter(|p| p.arch == path.arch || p.arch == "all")
        .collect();

    let text = super::deb::generate_packages(&arch_packages, &path.component);

    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let compressed = encoder.finish()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/gzip")
        .body(Body::from(compressed))
        .unwrap())
}

#[derive(serde::Deserialize)]
struct DebPoolPath {
    org: String,
    #[allow(dead_code)]
    env: String,
    #[allow(dead_code)]
    cv: String,
    product: String,
    repo: String,
    #[allow(dead_code)]
    component: String,
    #[allow(dead_code)]
    prefix: String,
    #[allow(dead_code)]
    source: String,
    filename: String,
}

async fn serve_deb_pool(
    State(state): State<ContentState>,
    Path(path): Path<DebPoolPath>,
) -> impl IntoResponse {
    let repo = match resolve_repo(&state.db, &path.org, &path.product, &path.repo) {
        Ok(r) => r,
        Err(e) => return Err(e),
    };

    let packages = match get_repo_packages(&state.db, &repo.id) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    // Find package by matching the filename in location_href
    let pkg = packages.iter()
        .find(|p| {
            p.location_href.ends_with(&path.filename) || p.deb_filename() == path.filename
        })
        .ok_or_else(|| (StatusCode::NOT_FOUND, "package not found".to_string()))?;

    // Try serving from local disk first
    if pkg.downloaded && !pkg.local_path.is_empty() {
        let local_file = std::path::PathBuf::from(&state.config.data_dir)
            .join("repos")
            .join(&repo.id)
            .join(&pkg.local_path);

        if let Ok(file) = tokio::fs::File::open(&local_file).await {
            let meta = file.metadata().await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            return Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/vnd.debian.binary-package")
                .header("Content-Length", meta.len().to_string())
                .body(body)
                .unwrap());
        }
    }

    // Fallback: proxy from upstream
    let base_url = repo.url.trim_end_matches('/');
    let pkg_url = format!("{}/{}", base_url, pkg.location_href);

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resp = client.get(&pkg_url).send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream fetch failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err((StatusCode::BAD_GATEWAY, format!("upstream returned {}", resp.status())));
    }

    let bytes = resp.bytes().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("upstream read failed: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/vnd.debian.binary-package")
        .header("Content-Length", bytes.len().to_string())
        .body(Body::from(bytes))
        .unwrap())
}
