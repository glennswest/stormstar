//! Host inventory page.

use axum::extract::State;
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
            </tr>"#,
            hostname = host.hostname, arch = host.arch, os = host.os,
            pkgs = host.installed_packages.len(),
        ));
    }

    let content = format!(
        r#"<div class="toolbar">
    <h1>Hosts</h1>
</div>
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
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>"#
    );

    Html(style::layout("Hosts", "hosts", &content))
}
