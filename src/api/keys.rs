//! Activation Key CRUD.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::ActivationKey;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/activation_keys", get(list).post(create))
        .route("/activation_keys/{id}", get(show).put(update).delete(delete))
}

#[derive(Deserialize)]
struct CreateKey {
    org_id: String,
    name: String,
    env_id: String,
    cv_id: String,
    max_hosts: Option<u64>,
}

#[derive(Deserialize)]
struct UpdateKey {
    name: Option<String>,
    env_id: Option<String>,
    cv_id: Option<String>,
    max_hosts: Option<u64>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ActivationKey>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<ActivationKey> = r.scan().primary()
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
) -> Result<Json<ActivationKey>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: ActivationKey = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("activation key not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateKey>,
) -> Result<(StatusCode, Json<ActivationKey>), AppError> {
    let mut key = ActivationKey::new(&body.org_id, &body.name, &body.env_id, &body.cv_id);
    key.max_hosts = body.max_hosts;

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(key.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(key)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateKey>,
) -> Result<Json<ActivationKey>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: ActivationKey = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("activation key not found"))?;

    let mut updated = old.clone();
    if let Some(name) = body.name { updated.name = name; }
    if let Some(env_id) = body.env_id { updated.env_id = env_id; }
    if let Some(cv_id) = body.cv_id { updated.cv_id = cv_id; }
    if let Some(max) = body.max_hosts { updated.max_hosts = Some(max); }

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: ActivationKey = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("activation key not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
