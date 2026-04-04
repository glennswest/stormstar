//! Errata listing (read-only, populated by sync).

use axum::{Router, routing::get};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/errata", get(list))
        .route("/errata/{id}", get(show))
}

async fn list() -> &'static str { "[]" }
async fn show() -> &'static str { "{}" }
