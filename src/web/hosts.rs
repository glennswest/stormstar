//! Host inventory page — list, delete.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let hosts: Vec<Host> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut rows = String::new();
    for host in &hosts {
        let errata_badge = if host.applicable_errata.is_empty() {
            r#"<span class="badge badge-green">Up to date</span>"#.to_string()
        } else {
            format!(r#"<span class="badge badge-yellow">{} applicable</span>"#,
                host.applicable_errata.len())
        };
        let checkin = host.last_checkin.as_deref().unwrap_or("never");
        rows.push_str(&format!(
            r#"<tr>
                <td>{hostname}</td>
                <td>{arch}</td>
                <td>{os}</td>
                <td>{pkgs}</td>
                <td>{errata_badge}</td>
                <td>{checkin}</td>
                <td>
                    <button class="sm danger" hx-post="/ui/hosts/{id}/delete" hx-swap="none"
                            hx-confirm="Delete host '{hostname}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            id = host.id, hostname = host.hostname, arch = host.arch, os = host.os,
            pkgs = host.installed_packages.len(),
        ));
    }

    let empty = if hosts.is_empty() {
        r#"<div class="empty-state"><p>No hosts registered. Use activation keys to register hosts via CLI.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Hosts</h1>
</div>
{empty}
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Hostname</th>
                <th>Arch</th>
                <th>OS</th>
                <th>Packages</th>
                <th>Errata</th>
                <th>Last Checkin</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Hosts", "hosts", &content))
}

pub async fn delete(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> StatusCode {
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let item: Host = match rw.get().primary(id) {
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
