//! Repository CRUD + sync trigger.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::{Repository, RepoSyncState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/repos", get(list).post(create))
        .route("/repos/{id}", get(show).put(update).delete(delete))
        .route("/repos/{id}/sync", axum::routing::post(sync))
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
}

fn default_content_type() -> String { "yum".to_string() }
fn default_arch() -> String { "x86_64".to_string() }

#[derive(Deserialize)]
struct UpdateRepo {
    name: Option<String>,
    url: Option<String>,
    content_type: Option<String>,
    arch: Option<String>,
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
    let mut repo = Repository::new(&body.product_id, &body.name, &body.url);
    repo.content_type = body.content_type;
    repo.arch = body.arch;

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
) -> Result<Json<Repository>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: Repository = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("repository not found"))?;

    let mut updated = old.clone();
    updated.sync_state = RepoSyncState::Syncing;

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    // TODO: spawn actual sync task
    Ok(Json(updated))
}
