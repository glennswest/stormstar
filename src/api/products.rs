//! Product CRUD.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::Product;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list).post(create))
        .route("/products/{id}", get(show).put(update).delete(delete))
}

#[derive(Deserialize)]
struct CreateProduct {
    org_id: String,
    name: String,
    label: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct UpdateProduct {
    name: Option<String>,
    description: Option<String>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Product>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<Product> = r.scan().primary()
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
) -> Result<Json<Product>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Product = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("product not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateProduct>,
) -> Result<(StatusCode, Json<Product>), AppError> {
    let mut product = Product::new(&body.org_id, &body.name, &body.label);
    product.description = body.description;

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(product.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(product)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProduct>,
) -> Result<Json<Product>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: Product = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("product not found"))?;

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
    let item: Product = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("product not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
