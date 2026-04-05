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
use axum::{Router, routing::get};
use native_db::Database;

use crate::config::Config;

#[derive(Clone)]
pub struct WebState {
    pub db: Arc<Database<'static>>,
    pub config: Arc<Config>,
}

pub fn routes() -> Router<WebState> {
    Router::new()
        .route("/", get(dashboard::page))
        .route("/ui/repos", get(repos::page))
        .route("/ui/views", get(views::page))
        .route("/ui/envs", get(envs::page))
        .route("/ui/hosts", get(hosts::page))
        .route("/ui/errata", get(errata::page))
        .route("/ui/keys", get(keys::page))
}
