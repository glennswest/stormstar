//! Activation keys page.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let keys: Vec<ActivationKey> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut rows = String::new();
    for key in &keys {
        let max = key.max_hosts.map(|m| m.to_string()).unwrap_or_else(|| "Unlimited".to_string());
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td><code>{key_val}</code></td>
                <td>{usage} / {max}</td>
                <td>{created}</td>
            </tr>"#,
            name = key.name, key_val = key.key,
            usage = key.usage_count, max = max,
            created = key.created_at,
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Activation Keys</h1>
</div>
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Key</th>
                <th>Usage</th>
                <th>Created</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Activation Keys", "keys", &content))
}
