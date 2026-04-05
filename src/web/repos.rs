//! Repos management page.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let repos: Vec<Repository> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut rows = String::new();
    for repo in &repos {
        let state_badge = match repo.sync_state {
            RepoSyncState::Synced => r#"<span class="badge badge-green">Synced</span>"#,
            RepoSyncState::Syncing => r#"<span class="badge badge-yellow">Syncing</span>"#,
            RepoSyncState::Failed => r#"<span class="badge badge-red">Failed</span>"#,
            RepoSyncState::NotSynced => r#"<span class="badge badge-dim">Not Synced</span>"#,
        };
        let last = repo.last_sync.as_deref().unwrap_or("never");
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{url}</td>
                <td>{state_badge}</td>
                <td>{pkgs}</td>
                <td>{last}</td>
                <td>
                    <button hx-post="/api/v1/repos/{id}/sync" hx-swap="none"
                            hx-on::after-request="location.reload()">Sync</button>
                </td>
            </tr>"#,
            id = repo.id, name = repo.name, url = repo.url,
            pkgs = repo.package_count, last = last,
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Repositories</h1>
</div>
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>URL</th>
                <th>Status</th>
                <th>Packages</th>
                <th>Last Sync</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Repositories", "repos", &content))
}
