//! Content View CRUD + publish/promote.

use axum::{Router, routing::get};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/content_views", get(list).post(create))
        .route("/content_views/{id}", get(show).put(update).delete(delete))
        .route("/content_views/{id}/publish", axum::routing::post(publish))
        .route("/content_views/{id}/promote", axum::routing::post(promote))
}

async fn list() -> &'static str { "[]" }
async fn show() -> &'static str { "{}" }
async fn create() -> &'static str { "{}" }
async fn update() -> &'static str { "{}" }
async fn delete() -> &'static str { "{}" }
async fn publish() -> &'static str { "{}" }
async fn promote() -> &'static str { "{}" }
