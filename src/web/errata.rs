//! Errata browser page.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let errata: Vec<Erratum> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut rows = String::new();
    for e in &errata {
        let type_badge = match e.erratum_type {
            ErratumType::Security => r#"<span class="badge badge-red">Security</span>"#,
            ErratumType::Bugfix => r#"<span class="badge badge-cyan">Bugfix</span>"#,
            ErratumType::Enhancement => r#"<span class="badge badge-purple">Enhancement</span>"#,
        };
        let severity_badge = match e.severity {
            ErratumSeverity::Critical => r#"<span class="badge badge-red">Critical</span>"#,
            ErratumSeverity::Important => r#"<span class="badge badge-yellow">Important</span>"#,
            ErratumSeverity::Moderate => r#"<span class="badge badge-cyan">Moderate</span>"#,
            ErratumSeverity::Low => r#"<span class="badge badge-dim">Low</span>"#,
            ErratumSeverity::None => r#"<span class="badge badge-dim">None</span>"#,
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{advisory_id}</td>
                <td>{title}</td>
                <td>{type_badge}</td>
                <td>{severity_badge}</td>
                <td>{cves}</td>
                <td>{issued}</td>
            </tr>"#,
            advisory_id = e.advisory_id, title = e.title,
            cves = if e.cves.is_empty() { "-".to_string() } else { e.cves.join(", ") },
            issued = e.issued,
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Errata</h1>
</div>
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Advisory</th>
                <th>Title</th>
                <th>Type</th>
                <th>Severity</th>
                <th>CVEs</th>
                <th>Issued</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Errata", "errata", &content))
}
