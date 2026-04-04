//! Sync Plan CRUD.

use axum::{Router, routing::get};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sync_plans", get(list).post(create))
        .route("/sync_plans/{id}", get(show).put(update).delete(delete))
}

async fn list() -> &'static str { "[]" }
async fn show() -> &'static str { "{}" }
async fn create() -> &'static str { "{}" }
async fn update() -> &'static str { "{}" }
async fn delete() -> &'static str { "{}" }
