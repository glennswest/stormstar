//! Repos management page — list, create, delete, sync.

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::db::models::*;

#[derive(Deserialize)]
pub struct CreateRepoForm {
    pub product_id: String,
    pub name: String,
    pub url: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub codename: Option<String>,
    #[serde(default)]
    pub components: Option<String>,
    #[serde(default)]
    pub architectures: Option<String>,
}

fn default_content_type() -> String { "yum".to_string() }

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let repos: Vec<Repository> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let products: Vec<Product> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut product_options = String::new();
    for p in &products {
        product_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = p.id, name = p.name
        ));
    }

    let mut rows = String::new();
    for repo in &repos {
        let state_badge = match repo.sync_state {
            RepoSyncState::Synced => r#"<span class="badge badge-green">Synced</span>"#,
            RepoSyncState::Syncing => r#"<span class="badge badge-yellow">Syncing</span>"#,
            RepoSyncState::Failed => r#"<span class="badge badge-red">Failed</span>"#,
            RepoSyncState::NotSynced => r#"<span class="badge badge-dim">Not Synced</span>"#,
        };
        let last = repo.last_sync.as_deref().unwrap_or("never");
        let url_display = if repo.url.len() > 60 {
            format!("{}...", &repo.url[..57])
        } else {
            repo.url.clone()
        };
        let type_badge = if repo.content_type == "deb" {
            r#"<span class="badge badge-cyan">deb</span>"#
        } else {
            r#"<span class="badge badge-purple">yum</span>"#
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{type_badge}</td>
                <td class="url-cell" title="{url}">{url_display}</td>
                <td>{state_badge}</td>
                <td>{pkgs}</td>
                <td>{errata}</td>
                <td>{last}</td>
                <td>
                    <button class="sm" hx-post="/api/v1/repos/{id}/sync" hx-swap="none"
                            hx-on::after-request="location.reload()">Sync</button>
                    <button class="sm danger" hx-post="/ui/repos/{id}/delete" hx-swap="none"
                            hx-confirm="Delete repository '{name}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            id = repo.id, name = repo.name,
            type_badge = type_badge,
            url = repo.url, url_display = url_display,
            pkgs = repo.package_count,
            errata = repo.errata_count,
            last = last,
        ));
    }

    let empty = if repos.is_empty() {
        r#"<div class="empty-state"><p>No repositories yet. Create one above to get started.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Repositories</h1>
    <details class="create-form">
        <summary><button>+ New Repository</button></summary>
        <div class="card">
            <form hx-post="/ui/repos/create" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) location.reload()">
                <div class="form-row">
                    <div class="form-group">
                        <label>Product</label>
                        <select name="product_id" required>{product_options}</select>
                    </div>
                    <div class="form-group">
                        <label>Type</label>
                        <select name="content_type" onchange="document.getElementById('deb-fields').style.display=this.value==='deb'?'flex':'none'">
                            <option value="yum">YUM (RPM)</option>
                            <option value="deb">APT (Deb)</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Name</label>
                        <input name="name" placeholder="e.g. CentOS Base" required>
                    </div>
                    <div class="form-group" style="flex:2">
                        <label>URL</label>
                        <input name="url" placeholder="https://mirror.centos.org/centos/8/BaseOS/x86_64/os/" required>
                    </div>
                    <div class="form-group" style="flex:0">
                        <label>&nbsp;</label>
                        <button type="submit">Create</button>
                    </div>
                </div>
                <div id="deb-fields" class="form-row" style="display:none">
                    <div class="form-group">
                        <label>Codename</label>
                        <input name="codename" placeholder="e.g. jammy, bookworm">
                    </div>
                    <div class="form-group">
                        <label>Components</label>
                        <input name="components" placeholder="e.g. main,restricted,universe">
                    </div>
                    <div class="form-group">
                        <label>Architectures</label>
                        <input name="architectures" placeholder="e.g. amd64,arm64">
                    </div>
                </div>
            </form>
        </div>
    </details>
</div>
{empty}
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Type</th>
                <th>URL</th>
                <th>Status</th>
                <th>Packages</th>
                <th>Errata</th>
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

pub async fn create(
    State(state): State<WebState>,
    Form(body): Form<CreateRepoForm>,
) -> StatusCode {
    let repo = if body.content_type == "deb" {
        Repository::new_deb(
            &body.product_id,
            &body.name,
            &body.url,
            body.codename.as_deref().unwrap_or("stable"),
            body.components.as_deref().unwrap_or("main"),
            body.architectures.as_deref().unwrap_or("amd64"),
        )
    } else {
        Repository::new(&body.product_id, &body.name, &body.url)
    };
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if rw.insert(repo).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if rw.commit().is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn delete(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> StatusCode {
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let item: Repository = match rw.get().primary(id) {
        Ok(Some(item)) => item,
        _ => return StatusCode::NOT_FOUND,
    };
    if rw.remove(item).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if rw.commit().is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}
