//! Sync Logs page — history of repository sync operations.

use axum::extract::State;
use axum::response::Html;

use super::WebState;
use super::style;
use super::relative_time;
use crate::db::models::*;

pub async fn page(State(state): State<WebState>) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let mut logs: Vec<SyncLog> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    // Sort by started_at descending (most recent first)
    logs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    // Limit to last 200 entries
    logs.truncate(200);

    let mut rows = String::new();
    for log in &logs {
        let status_badge = match log.status.as_str() {
            "success" => r#"<span class="badge badge-green">Success</span>"#,
            "failed" => r#"<span class="badge badge-red">Failed</span>"#,
            "started" => r#"<span class="badge badge-yellow">In Progress</span>"#,
            _ => r#"<span class="badge badge-dim">Unknown</span>"#,
        };

        let started = relative_time(&log.started_at);
        let duration = if let Some(ref finished) = log.finished_at {
            if let (Ok(s), Ok(f)) = (
                chrono::DateTime::parse_from_rfc3339(&log.started_at),
                chrono::DateTime::parse_from_rfc3339(finished),
            ) {
                let dur = f.signed_duration_since(s);
                let secs = dur.num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                }
            } else {
                "-".to_string()
            }
        } else if log.status == "started" {
            "running...".to_string()
        } else {
            "-".to_string()
        };

        let message_display = if log.message.len() > 80 {
            format!("{}...", &log.message[..77])
        } else {
            log.message.clone()
        };

        rows.push_str(&format!(
            r#"<tr>
                <td>{started}</td>
                <td>{repo_name}</td>
                <td>{status_badge}</td>
                <td>{pkgs}</td>
                <td>{errata}</td>
                <td>{duration}</td>
                <td title="{message}">{message_display}</td>
            </tr>"#,
            started = started,
            repo_name = log.repo_name,
            status_badge = status_badge,
            pkgs = log.packages_synced,
            errata = log.errata_synced,
            duration = duration,
            message = log.message,
            message_display = message_display,
        ));
    }

    let empty = if logs.is_empty() {
        r#"<div class="empty-state"><p>No sync logs yet. Sync a repository to see history here.</p></div>"#
    } else { "" };

    let content = format!(
        r#"<div class="toolbar">
    <h1>Sync Logs</h1>
</div>
{empty}
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Time</th>
                <th>Repository</th>
                <th>Status</th>
                <th>Packages</th>
                <th>Errata</th>
                <th>Duration</th>
                <th>Message</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Sync Logs", "logs", &content))
}
