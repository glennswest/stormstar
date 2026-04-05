//! Errata browser page — list with sync button.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;

use super::WebState;
use super::style;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let errata: Vec<Erratum> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let security_count = errata.iter().filter(|e| e.erratum_type == ErratumType::Security).count();
    let bugfix_count = errata.iter().filter(|e| e.erratum_type == ErratumType::Bugfix).count();
    let enhancement_count = errata.iter().filter(|e| e.erratum_type == ErratumType::Enhancement).count();

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

    let empty = if errata.is_empty() {
        r#"<div class="empty-state"><p>No errata synced yet. Sync repositories first, or use the Sync Errata button.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Errata</h1>
    <div class="actions">
        <button hx-post="/ui/errata/sync" hx-swap="none"
                hx-on::after-request="if(event.detail.successful) location.reload()">Sync Errata</button>
    </div>
</div>
<div class="grid" style="margin-bottom:1rem">
    <div class="card stat">
        <div class="value">{total}</div>
        <div class="label">Total Errata</div>
    </div>
    <div class="card stat">
        <div class="value" style="color:var(--red)">{security}</div>
        <div class="label">Security</div>
    </div>
    <div class="card stat">
        <div class="value" style="color:var(--cyan)">{bugfix}</div>
        <div class="label">Bugfix</div>
    </div>
    <div class="card stat">
        <div class="value" style="color:var(--purple)">{enhancement}</div>
        <div class="label">Enhancement</div>
    </div>
</div>
{empty}
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
</div>"#,
        total = errata.len(),
        security = security_count,
        bugfix = bugfix_count,
        enhancement = enhancement_count,
    );

    Html(style::layout("Errata", "errata", &content))
}

pub async fn sync(State(state): State<WebState>) -> StatusCode {
    let db = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::content::errata::sync_all_errata(&db).await {
            tracing::error!("Errata sync failed: {}", e);
        }
    });
    StatusCode::OK
}
