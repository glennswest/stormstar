//! HTTP yum-compatible repo serving.
//!
//! Serves repository content at:
//!   /pulp/repos/<org_label>/<env_label>/<cv_label>/custom/<product_label>/<repo_label>/
//!
//! Directory structure:
//!   repodata/repomd.xml          — generated from DB
//!   repodata/primary.xml.gz      — generated from DB
//!   Packages/<first_letter>/<filename>.rpm — proxied from upstream

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

use crate::config::Config;
use crate::db::models::*;

#[derive(Clone)]
pub struct ContentState {
    pub db: Arc<Database<'static>>,
    pub config: Arc<Config>,
}

pub fn routes() -> Router<ContentState> {
    Router::new()
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/repodata/repomd.xml",
            get(serve_repomd))
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/repodata/primary.xml.gz",
            get(serve_primary))
        .route("/pulp/repos/{org}/{env}/{cv}/custom/{product}/{repo}/Packages/{letter}/{filename}",
            get(serve_package))
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

    // Proxy the package from upstream
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
