//! Web UI — inline HTML, HTMX, Dracula dark theme.

pub mod style;
pub mod dashboard;
pub mod repos;
pub mod views;
pub mod envs;
pub mod hosts;
pub mod errata;
pub mod keys;

use std::sync::Arc;
use axum::{Router, routing::{get, post}};
use native_db::Database;

use crate::config::Config;

#[derive(Clone)]
pub struct WebState {
    pub db: Arc<Database<'static>>,
    pub config: Arc<Config>,
}

pub fn routes() -> Router<WebState> {
    Router::new()
        // Pages
        .route("/", get(dashboard::page))
        .route("/ui/repos", get(repos::page))
        .route("/ui/views", get(views::page))
        .route("/ui/envs", get(envs::page))
        .route("/ui/hosts", get(hosts::page))
        .route("/ui/errata", get(errata::page))
        .route("/ui/keys", get(keys::page))
        // Repo actions
        .route("/ui/repos/create", post(repos::create))
        .route("/ui/repos/{id}/delete", post(repos::delete))
        // Content view actions
        .route("/ui/views/create", post(views::create))
        .route("/ui/views/{id}/delete", post(views::delete))
        // Environment actions
        .route("/ui/envs/create", post(envs::create))
        .route("/ui/envs/{id}/delete", post(envs::delete))
        // Host actions
        .route("/ui/hosts/{id}/delete", post(hosts::delete))
        // Errata actions
        .route("/ui/errata/sync", post(errata::sync))
        // Key actions
        .route("/ui/keys/create", post(keys::create))
        .route("/ui/keys/{id}/delete", post(keys::delete))
}
