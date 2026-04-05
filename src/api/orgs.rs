//! Organization CRUD.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::Organization;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/organizations", get(list).post(create))
        .route("/organizations/{id}", get(show).put(update).delete(delete))
}

#[derive(Deserialize)]
struct CreateOrg {
    name: String,
    label: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct UpdateOrg {
    name: Option<String>,
    description: Option<String>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Organization>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<Organization> = r.scan().primary()
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
) -> Result<Json<Organization>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Organization = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("organization not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateOrg>,
) -> Result<(StatusCode, Json<Organization>), AppError> {
    let mut org = Organization::new(&body.name, &body.label);
    org.description = body.description;

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(org.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(org)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateOrg>,
) -> Result<Json<Organization>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: Organization = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("organization not found"))?;

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
    let item: Organization = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("organization not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
