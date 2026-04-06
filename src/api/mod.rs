//! REST API — axum router, AppState, error types.

pub mod orgs;
pub mod products;
pub mod repos;
pub mod views;
pub mod envs;
pub mod hosts;
pub mod errata;
pub mod keys;
pub mod plans;

use std::sync::Arc;

use axum::{
    Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json,
};
use native_db::Database;
use serde::Serialize;

use crate::config::Config;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database<'static>>,
    pub config: Arc<Config>,
}

/// API error type.
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: msg.into() }
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: msg.into() }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: msg.into() }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: self.message };
        (self.status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::internal(e.to_string())
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: "0.3.0",
    })
}

/// Build the full API router.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .merge(orgs::routes())
        .merge(products::routes())
        .merge(repos::routes())
        .merge(views::routes())
        .merge(envs::routes())
        .merge(hosts::routes())
        .merge(errata::routes())
        .merge(keys::routes())
        .merge(plans::routes());

    Router::new()
        .nest("/api/v1", api)
        .with_state(state)
}
