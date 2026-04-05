//! Content views page.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let cvs: Vec<ContentView> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut rows = String::new();
    for cv in &cvs {
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{label}</td>
                <td>{repos}</td>
                <td>{version}</td>
                <td>
                    <button hx-post="/api/v1/content_views/{id}/publish" hx-swap="none"
                            hx-on::after-request="location.reload()">Publish</button>
                </td>
            </tr>"#,
            name = cv.name, label = cv.label,
            repos = cv.repo_ids.len(),
            version = cv.latest_version,
            id = cv.id,
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Content Views</h1>
</div>
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
