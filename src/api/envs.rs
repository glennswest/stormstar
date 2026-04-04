//! Lifecycle Environment CRUD.

use axum::{Router, routing::get};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/environments", get(list).post(create))
        .route("/environments/{id}", get(show).put(update).delete(delete))
}

async fn list() -> &'static str { "[]" }
async fn show() -> &'static str { "{}" }
async fn create() -> &'static str { "{}" }
async fn update() -> &'static str { "{}" }
async fn delete() -> &'static str { "{}" }
