//! Repository CRUD + sync trigger.

use axum::{Router, routing::get};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/repos", get(list).post(create))
        .route("/repos/{id}", get(show).put(update).delete(delete))
        .route("/repos/{id}/sync", axum::routing::post(sync))
}

async fn list() -> &'static str { "[]" }
async fn show() -> &'static str { "{}" }
async fn create() -> &'static str { "{}" }
async fn update() -> &'static str { "{}" }
async fn delete() -> &'static str { "{}" }
async fn sync() -> &'static str { "{}" }
