//! Activation keys page — list, create, delete.

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::db::models::*;

#[derive(Deserialize)]
pub struct CreateKeyForm {
    pub org_id: String,
    pub name: String,
    pub env_id: String,
    pub cv_id: String,
}

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let keys: Vec<ActivationKey> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let orgs: Vec<Organization> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let envs: Vec<LifecycleEnvironment> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let cvs: Vec<ContentView> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut org_options = String::new();
    for o in &orgs {
        org_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = o.id, name = o.name
        ));
    }

    let mut env_options = String::new();
    for e in &envs {
        env_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = e.id, name = e.name
        ));
    }

    let mut cv_options = String::new();
    for c in &cvs {
        cv_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = c.id, name = c.name
        ));
    }

    let mut rows = String::new();
    for key in &keys {
        let max = key.max_hosts.map(|m| m.to_string()).unwrap_or_else(|| "Unlimited".to_string());
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td><code>{key_val}</code></td>
                <td>{usage} / {max}</td>
                <td>{created}</td>
                <td>
                    <button class="sm danger" hx-post="/ui/keys/{id}/delete" hx-swap="none"
                            hx-confirm="Delete activation key '{name}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            id = key.id, name = key.name, key_val = key.key,
            usage = key.usage_count, max = max,
            created = key.created_at,
        ));
    }

    let empty = if keys.is_empty() {
        r#"<div class="empty-state"><p>No activation keys yet. Create one to enable host registration.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Activation Keys</h1>
    <details class="create-form">
        <summary><button>+ New Key</button></summary>
        <div class="card">
            <form hx-post="/ui/keys/create" hx-swap="none"
                  hx-on::after-request="if(event.detail.successful) location.reload()">
                <div class="form-row">
                    <div class="form-group">
                        <label>Organization</label>
                        <select name="org_id" required>{org_options}</select>
                    </div>
                    <div class="form-group">
                        <label>Name</label>
                        <input name="name" placeholder="e.g. dev-servers" required>
                    </div>
                    <div class="form-group">
                        <label>Environment</label>
                        <select name="env_id" required>{env_options}</select>
                    </div>
                    <div class="form-group">
                        <label>Content View</label>
                        <select name="cv_id" required>{cv_options}</select>
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
                <th>Key</th>
                <th>Usage</th>
                <th>Created</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Activation Keys", "keys", &content))
}

pub async fn create(
    State(state): State<WebState>,
    Form(body): Form<CreateKeyForm>,
) -> StatusCode {
    let key = ActivationKey::new(&body.org_id, &body.name, &body.env_id, &body.cv_id);
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if rw.insert(key).is_err() {
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
    let item: ActivationKey = match rw.get().primary(id) {
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
