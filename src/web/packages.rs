//! Package browser page — search, filter by arch, paginated table.

use axum::extract::{Path, Query, State};
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::content::download;
use crate::db::models::*;

#[derive(Deserialize)]
pub struct PackageQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default = "default_page")]
    pub page: u64,
}

fn default_page() -> u64 { 1 }

const PER_PAGE: u64 = 50;

pub async fn page(
    State(state): State<WebState>,
    Path(repo_id): Path<String>,
    Query(query): Query<PackageQuery>,
) -> Html<String> {
    let r = state.db.r_transaction().unwrap();

    let repo: Option<Repository> = r.get().primary(repo_id.clone()).unwrap_or(None);
    let repo = match repo {
        Some(r) => r,
        None => return Html(style::layout("Not Found", "repos", "<h1>Repository not found</h1>")),
    };

    // Get all packages for this repo
    let all_pkgs: Vec<Package> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut repo_pkgs: Vec<Package> = all_pkgs.into_iter()
        .filter(|p| p.repo_id == repo_id)
        .collect();

    // Collect unique architectures for filter dropdown
    let mut archs: Vec<String> = repo_pkgs.iter()
        .map(|p| p.arch.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    archs.sort();

    // Apply search filter
    if !query.q.is_empty() {
        let q_lower = query.q.to_lowercase();
        repo_pkgs.retain(|p| p.name.to_lowercase().contains(&q_lower));
    }

    // Apply arch filter
    if !query.arch.is_empty() {
        repo_pkgs.retain(|p| p.arch == query.arch);
    }

    // Sort by name
    repo_pkgs.sort_by(|a, b| a.name.cmp(&b.name));

    let total = repo_pkgs.len() as u64;
    let total_pages = total.div_ceil(PER_PAGE);
    let current_page = query.page.max(1).min(total_pages.max(1));
    let skip = (current_page - 1) * PER_PAGE;

    let page_pkgs: Vec<&Package> = repo_pkgs.iter()
        .skip(skip as usize)
        .take(PER_PAGE as usize)
        .collect();

    // Build arch dropdown options
    let mut arch_options = String::from(r#"<option value="">All Architectures</option>"#);
    for a in &archs {
        let selected = if *a == query.arch { " selected" } else { "" };
        arch_options.push_str(&format!(
            r#"<option value="{a}"{selected}>{a}</option>"#
        ));
    }

    // Build rows
    let mut rows = String::new();
    for pkg in &page_pkgs {
        let source_badge = if pkg.downloaded {
            r#"<span class="badge badge-green">Local</span>"#
        } else {
            r#"<span class="badge badge-dim">Upstream</span>"#
        };
        let size_str = download::format_bytes(pkg.size);
        let version_display = if pkg.epoch != "0" {
            format!("{}:{}-{}", pkg.epoch, pkg.version, pkg.release)
        } else if pkg.release.is_empty() {
            pkg.version.clone()
        } else {
            format!("{}-{}", pkg.version, pkg.release)
        };

        rows.push_str(&format!(
            r#"<tr>
                <td>{name}</td>
                <td>{version}</td>
                <td>{arch}</td>
                <td>{size}</td>
                <td>{source}</td>
            </tr>"#,
            name = pkg.name,
            version = version_display,
            arch = pkg.arch,
            size = size_str,
            source = source_badge,
        ));
    }

    // Build pagination
    let mut pagination = String::new();
    if total_pages > 1 {
        pagination.push_str(r#"<div style="display:flex;gap:0.5rem;justify-content:center;margin-top:1rem;align-items:center">"#);

        let base_q = format!("q={}&arch={}", query.q, query.arch);

        if current_page > 1 {
            pagination.push_str(&format!(
                r#"<a href="/ui/repos/{}/packages?{}&page={}" class="btn" style="text-decoration:none">Prev</a>"#,
                repo_id, base_q, current_page - 1
            ));
        }

        pagination.push_str(&format!(
            r#"<span style="color:var(--fg-dim)">Page {} of {}</span>"#,
            current_page, total_pages
        ));

        if current_page < total_pages {
            pagination.push_str(&format!(
                r#"<a href="/ui/repos/{}/packages?{}&page={}" class="btn" style="text-decoration:none">Next</a>"#,
                repo_id, base_q, current_page + 1
            ));
        }

        pagination.push_str("</div>");
    }

    let total_size: u64 = repo_pkgs.iter().map(|p| p.size).sum();
    let downloaded_count = repo_pkgs.iter().filter(|p| p.downloaded).count();

    let content = format!(
        r##"<div class="toolbar">
    <h1><a href="/ui/repos" style="color:var(--fg-dim)">Repositories</a> / {repo_name}</h1>
</div>
<div class="card" style="margin-bottom:1rem">
    <div style="display:flex;gap:2rem">
        <div><span style="color:var(--fg-dim)">Total Packages:</span> <strong>{total}</strong></div>
        <div><span style="color:var(--fg-dim)">Total Size:</span> <strong>{total_size}</strong></div>
        <div><span style="color:var(--fg-dim)">Downloaded:</span> <strong>{downloaded_count}</strong></div>
    </div>
</div>
<div class="card">
    <form method="get" action="/ui/repos/{repo_id}/packages" style="display:flex;gap:1rem;margin-bottom:1rem;align-items:end">
        <div class="form-group" style="flex:2">
            <label>Search</label>
            <input name="q" value="{search_q}" placeholder="Search by package name...">
        </div>
        <div class="form-group" style="flex:1">
            <label>Architecture</label>
            <select name="arch">{arch_options}</select>
        </div>
        <div class="form-group" style="flex:0">
            <label>&nbsp;</label>
            <button type="submit">Filter</button>
        </div>
    </form>
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Version</th>
                <th>Arch</th>
                <th>Size</th>
                <th>Source</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
    {pagination}
</div>"##,
        repo_name = repo.name,
        total = total,
        total_size = download::format_bytes(total_size),
        downloaded_count = downloaded_count,
        repo_id = repo_id,
        search_q = query.q,
        arch_options = arch_options,
        rows = rows,
        pagination = pagination,
    );

    Html(style::layout(&format!("Packages — {}", repo.name), "repos", &content))
}
