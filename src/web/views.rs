//! Content views page — list, create, delete, publish.

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::db::models::*;

#[derive(Deserialize)]
pub struct CreateViewForm {
    pub org_id: String,
    pub name: String,
}

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let cvs: Vec<ContentView> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let orgs: Vec<Organization> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut org_options = String::new();
    for o in &orgs {
        org_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = o.id, name = o.name
        ));
    }

    let mut rows = String::new();
    for cv in &cvs {
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{label}</td>
                <td>{repos}</td>
                <td>{version}</td>
                <td>
                    <button class="sm" hx-post="/api/v1/content_views/{id}/publish" hx-swap="none"
                            hx-on::after-request="location.reload()">Publish</button>
                    <button class="sm danger" hx-post="/ui/views/{id}/delete" hx-swap="none"
                            hx-confirm="Delete content view '{name}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            name = cv.name, label = cv.label,
            repos = cv.repo_ids.len(),
            version = cv.latest_version,
            id = cv.id,
        ));
    }

    let empty = if cvs.is_empty() {
        r#"<div class="empty-state"><p>No content views yet. Create one to start composing content.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Content Views</h1>
    <details class="create-form">
        <summary><button>+ New Content View</button></summary>
        <div class="card">
            <form hx-post="/ui/views/create" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) location.reload()">
                <div class="form-row">
                    <div class="form-group">
                        <label>Organization</label>
                        <select name="org_id" required>{org_options}</select>
                    </div>
                    <div class="form-group">
                        <label>Name</label>
                        <input name="name" placeholder="e.g. Base OS" required>
                    </div>
                    <div class="form-group" style="flex:0">
                        <label>&nbsp;</label>
                        <button type="submit">Create</button>
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
                <th>Label</th>
                <th>Repos</th>
                <th>Latest Version</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Content Views", "views", &content))
}

pub async fn create(
    State(state): State<WebState>,
    Form(body): Form<CreateViewForm>,
) -> StatusCode {
    let cv = ContentView::new(&body.org_id, &body.name);
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if rw.insert(cv).is_err() {
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
    let item: ContentView = match rw.get().primary(id) {
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
