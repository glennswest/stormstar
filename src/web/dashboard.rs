//! Dashboard page — overview stats.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let repo_count = r.scan().primary::<Repository>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let pkg_count = r.scan().primary::<Package>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let host_count = r.scan().primary::<Host>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let errata_count = r.scan().primary::<Erratum>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let cv_count = r.scan().primary::<ContentView>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let env_count = r.scan().primary::<LifecycleEnvironment>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);
    let key_count = r.scan().primary::<ActivationKey>()
        .map(|s| s.all().map(|a| a.count()).unwrap_or(0)).unwrap_or(0);

    let content = format!(
        r#"<h1>Dashboard</h1>
<div class="grid">
    <a href="/ui/repos" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{repo_count}</div>
            <div class="label">Repositories</div>
        </div>
    </a>
    <div class="card stat">
        <div class="value">{pkg_count}</div>
        <div class="label">Packages</div>
    </div>
    <a href="/ui/hosts" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{host_count}</div>
            <div class="label">Hosts</div>
        </div>
    </a>
    <a href="/ui/errata" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{errata_count}</div>
            <div class="label">Errata</div>
        </div>
    </a>
    <a href="/ui/views" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{cv_count}</div>
            <div class="label">Content Views</div>
        </div>
    </a>
    <a href="/ui/envs" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{env_count}</div>
            <div class="label">Environments</div>
        </div>
    </a>
    <a href="/ui/keys" style="text-decoration:none">
        <div class="card stat">
            <div class="value">{key_count}</div>
            <div class="label">Activation Keys</div>
        </div>
    </a>
</div>"#
    );

    Html(style::layout("Dashboard", "dashboard", &content))
}
