//! Lifecycle environments page.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let mut envs: Vec<LifecycleEnvironment> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    envs.sort_by_key(|e| e.position);

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
            </tr>"#,
            name = env.name, label = env.label,
            position = env.position,
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Lifecycle Environments</h1>
</div>
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Label</th>
                <th>Position</th>
                <th>Content</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Environments", "envs", &content))
}
