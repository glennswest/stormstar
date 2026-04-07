//! Web UI — inline HTML, HTMX, dark gray theme.

pub mod style;
pub mod dashboard;
pub mod repos;
pub mod views;
pub mod envs;
pub mod hosts;
pub mod errata;
pub mod keys;
pub mod logs;

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
        .route("/ui/logs", get(logs::page))
        // Repo actions
        .route("/ui/repos/create", post(repos::create))
        .route("/ui/repos/create-batch", post(repos::create_batch))
        .route("/ui/repos/{id}/toggle", post(repos::toggle))
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

/// Format an RFC3339 timestamp as relative time in America/Chicago (Central).
/// Examples: "just now", "5 minutes ago", "2 hours ago", "yesterday", "3 days ago"
pub fn relative_time(rfc3339: &str) -> String {
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(rfc3339) else {
        return rfc3339.to_string();
    };
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(parsed);

    let secs = duration.num_seconds();
    if secs < 0 {
        return "just now".to_string();
    }

    if secs < 60 {
        return "just now".to_string();
    }
    let mins = secs / 60;
    if mins < 60 {
        return if mins == 1 { "1 minute ago".to_string() } else { format!("{} minutes ago", mins) };
    }
    let hours = mins / 60;
    if hours < 24 {
        return if hours == 1 { "1 hour ago".to_string() } else { format!("{} hours ago", hours) };
    }
    let days = hours / 24;
    if days == 1 {
        return "yesterday".to_string();
    }
    if days < 30 {
        return format!("{} days ago", days);
    }
    let months = days / 30;
    if months < 12 {
        return if months == 1 { "1 month ago".to_string() } else { format!("{} months ago", months) };
    }

    // Fall back to a formatted date in Central time
    let central = chrono::FixedOffset::west_opt(6 * 3600).unwrap(); // CDT is -5, CST is -6; use -6 as safe default
    let local = parsed.with_timezone(&central);
    local.format("%b %d, %Y %l:%M %p").to_string()
}
