//! Repository CRUD + sync trigger.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::content::download;
use crate::db::models::{Repository, Package};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/repos", get(list).post(create))
        .route("/repos/{id}", get(show).put(update).delete(delete))
        .route("/repos/{id}/sync", axum::routing::post(sync))
        .route("/repos/{id}/packages", get(packages))
        .route("/repos/{id}/sync-progress", get(sync_progress))
        .route("/repos/{id}/size-estimate", get(size_estimate))
}

#[derive(Deserialize)]
struct CreateRepo {
    product_id: String,
    name: String,
    url: String,
    #[serde(default = "default_content_type")]
    content_type: String,
    #[serde(default = "default_arch")]
    arch: String,
    codename: Option<String>,
    components: Option<String>,
    architectures: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    ssl_client_cert: Option<String>,
    #[serde(default)]
    ssl_client_key: Option<String>,
}

fn default_content_type() -> String { "yum".to_string() }
fn default_arch() -> String { "x86_64".to_string() }

#[derive(Deserialize)]
struct UpdateRepo {
    name: Option<String>,
    url: Option<String>,
    content_type: Option<String>,
    arch: Option<String>,
    codename: Option<String>,
    components: Option<String>,
    architectures: Option<String>,
    enabled: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    ssl_client_cert: Option<String>,
    ssl_client_key: Option<String>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Repository>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<Repository> = r.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(items))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Repository>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Repository = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateRepo>,
) -> Result<(StatusCode, Json<Repository>), AppError> {
    let mut repo = if body.content_type == "deb" {
        Repository::new_deb(
            &body.product_id,
            &body.name,
            &body.url,
            body.codename.as_deref().unwrap_or("stable"),
            body.components.as_deref().unwrap_or("main"),
            body.architectures.as_deref().unwrap_or("amd64"),
        )
    } else {
        let mut r = Repository::new(&body.product_id, &body.name, &body.url);
        r.content_type = body.content_type;
        r.arch = body.arch;
        r
    };

    if let Some(enabled) = body.enabled { repo.enabled = enabled; }
    if let Some(u) = body.username { if !u.is_empty() { repo.username = Some(u); } }
    if let Some(p) = body.password { if !p.is_empty() { repo.password = Some(p); } }
    if let Some(c) = body.ssl_client_cert { if !c.is_empty() { repo.ssl_client_cert = Some(c); } }
    if let Some(k) = body.ssl_client_key { if !k.is_empty() { repo.ssl_client_key = Some(k); } }

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(repo.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(repo)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRepo>,
) -> Result<Json<Repository>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: Repository = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;

    let mut updated = old.clone();
    if let Some(name) = body.name { updated.name = name; }
    if let Some(url) = body.url { updated.url = url; }
    if let Some(ct) = body.content_type { updated.content_type = ct; }
    if let Some(arch) = body.arch { updated.arch = arch; }
    if let Some(cn) = body.codename { updated.codename = Some(cn); }
    if let Some(comp) = body.components { updated.components = Some(comp); }
    if let Some(archs) = body.architectures { updated.architectures = Some(archs); }
    if let Some(enabled) = body.enabled { updated.enabled = enabled; }
    if let Some(u) = body.username { updated.username = if u.is_empty() { None } else { Some(u) }; }
    if let Some(p) = body.password { updated.password = if p.is_empty() { None } else { Some(p) }; }
    if let Some(c) = body.ssl_client_cert { updated.ssl_client_cert = if c.is_empty() { None } else { Some(c) }; }
    if let Some(k) = body.ssl_client_key { updated.ssl_client_key = if k.is_empty() { None } else { Some(k) }; }

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Repository = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn sync(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify repo exists
    {
        let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
        let _: Repository = r.get().primary(id.clone())
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::not_found("repository not found"))?;
    }

    // Spawn sync in background
    let db = state.db.clone();
    let config = state.config.clone();
    let progress = state.progress.clone();
    let repo_id = id.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::content::repo::sync_repo(&db, &repo_id, &config, &progress).await {
            tracing::error!("Sync failed for {}: {}", repo_id, e);
        }
    });

    Ok(Json(serde_json::json!({
        "status": "syncing",
        "repo_id": id,
    })))
}

async fn packages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Package>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;

    // Verify repo exists
    let _: Repository = r.get().primary(id.clone())
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;

    let all_pkgs: Vec<Package> = r.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let repo_pkgs: Vec<Package> = all_pkgs.into_iter()
        .filter(|p| p.repo_id == id)
        .collect();

    Ok(Json(repo_pkgs))
}

async fn sync_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let map = state.progress.lock().await;
    if let Some(progress) = map.get(&id) {
        Ok(Json(serde_json::to_value(progress).unwrap()))
    } else {
        Ok(Json(serde_json::json!({
            "phase": "idle",
            "total_packages": 0,
            "downloaded": 0,
            "skipped": 0,
            "failed": 0,
            "bytes_downloaded": 0,
            "total_size_bytes": 0,
            "current_package": ""
        })))
    }
}

async fn size_estimate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;

    let repo: Repository = r.get().primary(id.clone())
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;

    let all_pkgs: Vec<Package> = r.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let repo_pkgs: Vec<&Package> = all_pkgs.iter()
        .filter(|p| p.repo_id == id)
        .collect();

    let total_size: u64 = repo_pkgs.iter().map(|p| p.size).sum();
    let downloaded_count = repo_pkgs.iter().filter(|p| p.downloaded).count() as u64;
    let downloaded_size: u64 = repo_pkgs.iter().filter(|p| p.downloaded).map(|p| p.download_size).sum();

    Ok(Json(serde_json::json!({
        "total_packages": repo.package_count,
        "total_size_bytes": total_size,
        "total_size_human": download::format_bytes(total_size),
        "downloaded_count": downloaded_count,
        "downloaded_size_bytes": downloaded_size,
        "downloaded_size_human": download::format_bytes(downloaded_size),
        "remaining_bytes": total_size.saturating_sub(downloaded_size),
        "remaining_human": download::format_bytes(total_size.saturating_sub(downloaded_size)),
    })))
}
