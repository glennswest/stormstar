//! Lifecycle environments page — list, create, delete.

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::db::models::*;

#[derive(Deserialize)]
pub struct CreateEnvForm {
    pub org_id: String,
    pub name: String,
    pub prior_id: Option<String>,
}

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let mut envs: Vec<LifecycleEnvironment> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    envs.sort_by_key(|e| e.position);

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

    let mut prior_options = String::from(r#"<option value="">— None (root) —</option>"#);
    for env in &envs {
        prior_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = env.id, name = env.name
        ));
    }

    let mut rows = String::new();
    for env in &envs {
        let cv_badge = match &env.cv_version_id {
            Some(_) => r#"<span class="badge badge-green">Published</span>"#,
            None => r#"<span class="badge badge-dim">Empty</span>"#,
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{label}</td>
                <td>{position}</td>
                <td>{cv_badge}</td>
                <td>
                    <button class="sm danger" hx-post="/ui/envs/{id}/delete" hx-swap="none"
                            hx-confirm="Delete environment '{name}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            id = env.id, name = env.name, label = env.label,
            position = env.position,
        ));
    }

    let empty = if envs.is_empty() {
        r#"<div class="empty-state"><p>No environments yet. Create a lifecycle chain to manage content promotion.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Lifecycle Environments</h1>
    <details class="create-form">
        <summary><button>+ New Environment</button></summary>
        <div class="card">
            <form hx-post="/ui/envs/create" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) location.reload()">
                <div class="form-row">
                    <div class="form-group">
                        <label>Organization</label>
                        <select name="org_id" required>{org_options}</select>
                    </div>
                    <div class="form-group">
                        <label>Name</label>
                        <input name="name" placeholder="e.g. Development" required>
                    </div>
                    <div class="form-group">
                        <label>Prior Environment</label>
                        <select name="prior_id">{prior_options}</select>
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
                <th>Position</th>
                <th>Content</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Environments", "envs", &content))
}

pub async fn create(
    State(state): State<WebState>,
    Form(body): Form<CreateEnvForm>,
) -> StatusCode {
    // Determine position from existing envs
    let position = {
        let r = match state.db.r_transaction() {
            Ok(r) => r,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };
        let envs: Vec<LifecycleEnvironment> = r.scan().primary()
            .unwrap_or_else(|_| panic!()).all().unwrap_or_else(|_| panic!())
            .collect::<Result<Vec<_>, _>>().unwrap_or_default();
        envs.len() as u32
    };

    let prior = body.prior_id.as_deref().filter(|s| !s.is_empty());
    let env = LifecycleEnvironment::new(&body.org_id, &body.name, position, prior);

    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if rw.insert(env).is_err() {
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
    let item: LifecycleEnvironment = match rw.get().primary(id) {
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
