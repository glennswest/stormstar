//! Lifecycle Environment CRUD.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::LifecycleEnvironment;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/environments", get(list).post(create))
        .route("/environments/{id}", get(show).put(update).delete(delete))
}

#[derive(Deserialize)]
struct CreateEnv {
    org_id: String,
    name: String,
    #[serde(default)]
    description: String,
    prior_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdateEnv {
    name: Option<String>,
    description: Option<String>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<LifecycleEnvironment>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<LifecycleEnvironment> = r.scan().primary()
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
) -> Result<Json<LifecycleEnvironment>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: LifecycleEnvironment = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("environment not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateEnv>,
) -> Result<(StatusCode, Json<LifecycleEnvironment>), AppError> {
    // Determine position based on prior
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;

    let position = if let Some(ref prior_id) = body.prior_id {
        let prior: LifecycleEnvironment = rw.get().primary(prior_id.clone())
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::bad_request("prior environment not found"))?;
        prior.position + 1
    } else {
        0
    };

    let mut env = LifecycleEnvironment::new(
        &body.org_id,
        &body.name,
        position,
        body.prior_id.as_deref(),
    );
    env.description = body.description;

    // Link prior's successor to this new env
    if let Some(ref prior_id) = body.prior_id {
        let old_prior: LifecycleEnvironment = rw.get().primary(prior_id.clone())
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::bad_request("prior environment not found"))?;
        let mut updated_prior = old_prior.clone();
        updated_prior.successor_id = Some(env.id.clone());
        rw.update(old_prior, updated_prior).map_err(|e| AppError::internal(e.to_string()))?;
    }

    rw.insert(env.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(env)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEnv>,
) -> Result<Json<LifecycleEnvironment>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: LifecycleEnvironment = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("environment not found"))?;

    let mut updated = old.clone();
    if let Some(name) = body.name { updated.name = name; }
    if let Some(desc) = body.description { updated.description = desc; }

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: LifecycleEnvironment = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("environment not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
