//! Content View CRUD + publish/promote.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, AppError};
use crate::db::models::{ContentView, ContentViewVersion, LifecycleEnvironment};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/content_views", get(list).post(create))
        .route("/content_views/{id}", get(show).put(update).delete(delete))
        .route("/content_views/{id}/publish", axum::routing::post(publish))
        .route("/content_views/{id}/promote", axum::routing::post(promote))
}

#[derive(Deserialize)]
struct CreateCv {
    org_id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    repo_ids: Vec<String>,
}

#[derive(Deserialize)]
struct UpdateCv {
    name: Option<String>,
    description: Option<String>,
    repo_ids: Option<Vec<String>>,
    filter_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct PromoteRequest {
    version: u32,
    env_id: String,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ContentView>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<ContentView> = r.scan().primary()
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
) -> Result<Json<ContentView>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: ContentView = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("content view not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateCv>,
) -> Result<(StatusCode, Json<ContentView>), AppError> {
    let mut cv = ContentView::new(&body.org_id, &body.name);
    cv.description = body.description;
    cv.repo_ids = body.repo_ids;

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(cv.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(cv)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCv>,
) -> Result<Json<ContentView>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: ContentView = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("content view not found"))?;

    let mut updated = old.clone();
    if let Some(name) = body.name { updated.name = name; }
    if let Some(desc) = body.description { updated.description = desc; }
    if let Some(repos) = body.repo_ids { updated.repo_ids = repos; }
    if let Some(filters) = body.filter_ids { updated.filter_ids = filters; }

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: ContentView = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("content view not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn publish(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<ContentViewVersion>), AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: ContentView = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("content view not found"))?;

    let new_version = old.latest_version + 1;

    // Count packages across all repos in the CV
    let mut package_count: u64 = 0;
    let mut errata_count: u64 = 0;
    for repo_id in &old.repo_ids {
        if let Some(repo) = rw.get().primary::<crate::db::models::Repository>(repo_id.clone())
            .map_err(|e| AppError::internal(e.to_string()))? {
            package_count += repo.package_count;
            errata_count += repo.errata_count;
        }
    }

    let version = ContentViewVersion {
        id: Uuid::new_v4().to_string(),
        cv_id: old.id.clone(),
        version: new_version,
        package_count,
        errata_count,
        repo_ids: old.repo_ids.clone(),
        published_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut updated_cv = old.clone();
    updated_cv.latest_version = new_version;

    rw.insert(version.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.update(old, updated_cv).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(version)))
}

async fn promote(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PromoteRequest>,
) -> Result<Json<LifecycleEnvironment>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;

    // Verify CV exists and version exists
    let _cv: ContentView = rw.get().primary(id.clone())
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("content view not found"))?;

    // Find the version
    let versions: Vec<ContentViewVersion> = rw.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let version = versions.iter()
        .find(|v| v.cv_id == id && v.version == body.version)
        .ok_or_else(|| AppError::not_found("content view version not found"))?;

    // Update the environment
    let old_env: LifecycleEnvironment = rw.get().primary(body.env_id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("environment not found"))?;

    let mut updated_env = old_env.clone();
    updated_env.cv_version_id = Some(version.id.clone());

    rw.update(old_env, updated_env.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated_env))
}
