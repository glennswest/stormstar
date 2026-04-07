//! Repos management page — catalog selector, list, create, delete, sync, toggle.

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use super::WebState;
use super::style;
use crate::db::models::*;
use crate::web::relative_time;

// ── Known Repo Catalog ──────────────────────────────────────────────

struct KnownRepo {
    distro: &'static str,
    name: &'static str,
    url: &'static str,
    content_type: &'static str,
    codename: Option<&'static str>,
    components: Option<&'static str>,
    architectures: Option<&'static str>,
    needs_auth: bool,
}

const KNOWN_REPOS: &[KnownRepo] = &[
    // CentOS 7
    KnownRepo { distro: "CentOS 7", name: "CentOS 7 - OS", url: "https://vault.centos.org/centos/7/os/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "CentOS 7", name: "CentOS 7 - Updates", url: "https://vault.centos.org/centos/7/updates/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "CentOS 7", name: "CentOS 7 - Extras", url: "https://vault.centos.org/centos/7/extras/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "CentOS 7", name: "CentOS 7 - SCL", url: "https://vault.centos.org/centos/7/sclo/x86_64/sclo/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // Rocky 8
    KnownRepo { distro: "Rocky 8", name: "Rocky 8 - BaseOS", url: "https://dl.rockylinux.org/pub/rocky/8/BaseOS/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 8", name: "Rocky 8 - AppStream", url: "https://dl.rockylinux.org/pub/rocky/8/AppStream/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 8", name: "Rocky 8 - PowerTools", url: "https://dl.rockylinux.org/pub/rocky/8/PowerTools/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 8", name: "Rocky 8 - Extras", url: "https://dl.rockylinux.org/pub/rocky/8/extras/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // Rocky 9
    KnownRepo { distro: "Rocky 9", name: "Rocky 9 - BaseOS", url: "https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 9", name: "Rocky 9 - AppStream", url: "https://dl.rockylinux.org/pub/rocky/9/AppStream/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 9", name: "Rocky 9 - CRB", url: "https://dl.rockylinux.org/pub/rocky/9/CRB/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 9", name: "Rocky 9 - Extras", url: "https://dl.rockylinux.org/pub/rocky/9/extras/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "Rocky 9", name: "Rocky 9 - Devel", url: "https://dl.rockylinux.org/pub/rocky/9/devel/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // AlmaLinux 8
    KnownRepo { distro: "AlmaLinux 8", name: "AlmaLinux 8 - BaseOS", url: "https://repo.almalinux.org/almalinux/8/BaseOS/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 8", name: "AlmaLinux 8 - AppStream", url: "https://repo.almalinux.org/almalinux/8/AppStream/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 8", name: "AlmaLinux 8 - PowerTools", url: "https://repo.almalinux.org/almalinux/8/PowerTools/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 8", name: "AlmaLinux 8 - Extras", url: "https://repo.almalinux.org/almalinux/8/extras/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // AlmaLinux 9
    KnownRepo { distro: "AlmaLinux 9", name: "AlmaLinux 9 - BaseOS", url: "https://repo.almalinux.org/almalinux/9/BaseOS/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 9", name: "AlmaLinux 9 - AppStream", url: "https://repo.almalinux.org/almalinux/9/AppStream/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 9", name: "AlmaLinux 9 - CRB", url: "https://repo.almalinux.org/almalinux/9/CRB/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "AlmaLinux 9", name: "AlmaLinux 9 - Extras", url: "https://repo.almalinux.org/almalinux/9/extras/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // RHEL 7
    KnownRepo { distro: "RHEL 7", name: "RHEL 7 - Server", url: "https://cdn.redhat.com/content/dist/rhel/server/7/7Server/x86_64/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 7", name: "RHEL 7 - Optional", url: "https://cdn.redhat.com/content/dist/rhel/server/7/7Server/x86_64/optional/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 7", name: "RHEL 7 - Extras", url: "https://cdn.redhat.com/content/dist/rhel/server/7/7Server/x86_64/extras/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    // RHEL 8
    KnownRepo { distro: "RHEL 8", name: "RHEL 8 - BaseOS", url: "https://cdn.redhat.com/content/dist/rhel8/8/x86_64/baseos/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 8", name: "RHEL 8 - AppStream", url: "https://cdn.redhat.com/content/dist/rhel8/8/x86_64/appstream/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 8", name: "RHEL 8 - CRB", url: "https://cdn.redhat.com/content/dist/rhel8/8/x86_64/codeready-builder/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    // RHEL 9
    KnownRepo { distro: "RHEL 9", name: "RHEL 9 - BaseOS", url: "https://cdn.redhat.com/content/dist/rhel9/9/x86_64/baseos/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 9", name: "RHEL 9 - AppStream", url: "https://cdn.redhat.com/content/dist/rhel9/9/x86_64/appstream/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    KnownRepo { distro: "RHEL 9", name: "RHEL 9 - CRB", url: "https://cdn.redhat.com/content/dist/rhel9/9/x86_64/codeready-builder/os/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: true },
    // EPEL
    KnownRepo { distro: "EPEL", name: "EPEL 7", url: "https://dl.fedoraproject.org/pub/epel/7/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "EPEL", name: "EPEL 8", url: "https://dl.fedoraproject.org/pub/epel/8/Everything/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    KnownRepo { distro: "EPEL", name: "EPEL 9", url: "https://dl.fedoraproject.org/pub/epel/9/Everything/x86_64/", content_type: "yum", codename: None, components: None, architectures: None, needs_auth: false },
    // Debian Bookworm
    KnownRepo { distro: "Debian Bookworm", name: "Debian Bookworm", url: "https://deb.debian.org/debian/", content_type: "deb", codename: Some("bookworm"), components: Some("main,contrib,non-free,non-free-firmware"), architectures: Some("amd64"), needs_auth: false },
    KnownRepo { distro: "Debian Bookworm", name: "Debian Bookworm Security", url: "https://deb.debian.org/debian-security/", content_type: "deb", codename: Some("bookworm-security"), components: Some("main,contrib,non-free,non-free-firmware"), architectures: Some("amd64"), needs_auth: false },
    KnownRepo { distro: "Debian Bookworm", name: "Debian Bookworm Updates", url: "https://deb.debian.org/debian/", content_type: "deb", codename: Some("bookworm-updates"), components: Some("main,contrib,non-free,non-free-firmware"), architectures: Some("amd64"), needs_auth: false },
    // Ubuntu Noble (24.04)
    KnownRepo { distro: "Ubuntu Noble", name: "Ubuntu Noble", url: "https://archive.ubuntu.com/ubuntu/", content_type: "deb", codename: Some("noble"), components: Some("main,restricted,universe,multiverse"), architectures: Some("amd64"), needs_auth: false },
    KnownRepo { distro: "Ubuntu Noble", name: "Ubuntu Noble Security", url: "https://security.ubuntu.com/ubuntu/", content_type: "deb", codename: Some("noble-security"), components: Some("main,restricted,universe,multiverse"), architectures: Some("amd64"), needs_auth: false },
    KnownRepo { distro: "Ubuntu Noble", name: "Ubuntu Noble Updates", url: "https://archive.ubuntu.com/ubuntu/", content_type: "deb", codename: Some("noble-updates"), components: Some("main,restricted,universe,multiverse"), architectures: Some("amd64"), needs_auth: false },
];

/// Get unique distro names for the dropdown.
fn distro_names() -> Vec<&'static str> {
    let mut names: Vec<&str> = Vec::new();
    for kr in KNOWN_REPOS {
        if !names.contains(&kr.distro) {
            names.push(kr.distro);
        }
    }
    names
}

// ── Form types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateRepoForm {
    pub product_id: String,
    pub name: String,
    pub url: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub codename: Option<String>,
    #[serde(default)]
    pub components: Option<String>,
    #[serde(default)]
    pub architectures: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

fn default_content_type() -> String { "yum".to_string() }

#[derive(Deserialize)]
pub struct BatchCreateForm {
    pub product_id: String,
    pub distro: String,
    #[serde(default)]
    pub repos: Vec<String>,  // indices into KNOWN_REPOS
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub ssl_client_cert: Option<String>,
    #[serde(default)]
    pub ssl_client_key: Option<String>,
}

// ── Page ────────────────────────────────────────────────────────────

pub async fn page(State(state): State<WebState>) -> Html<String> {
    // Auto-create a default product and org if none exist
    {
        let r = state.db.r_transaction().unwrap();
        let products: Vec<Product> = r.scan().primary()
            .unwrap().all().unwrap()
            .collect::<Result<Vec<_>, _>>().unwrap_or_default();
        if products.is_empty() {
            drop(r);
            let rw = state.db.rw_transaction().unwrap();
            let orgs: Vec<Organization> = rw.scan().primary()
                .unwrap().all().unwrap()
                .collect::<Result<Vec<_>, _>>().unwrap_or_default();
            let org_id = if let Some(org) = orgs.first() {
                org.id.clone()
            } else {
                let org = Organization::new("Default", "default");
                let id = org.id.clone();
                let _ = rw.insert(org);
                id
            };
            let _ = rw.insert(Product::new(&org_id, "Default", "default"));
            let _ = rw.commit();
        }
    }

    let r = state.db.r_transaction().unwrap();

    let repos: Vec<Repository> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let products: Vec<Product> = r.scan().primary()
        .unwrap().all().unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap_or_default();

    let mut product_options = String::new();
    for p in &products {
        product_options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = p.id, name = p.name
        ));
    }

    // Build distro dropdown options
    let mut distro_options = String::new();
    for d in distro_names() {
        distro_options.push_str(&format!(r#"<option value="{d}">{d}</option>"#));
    }

    // Build JSON catalog for JS
    let mut catalog_js = String::from("[");
    for (i, kr) in KNOWN_REPOS.iter().enumerate() {
        if i > 0 { catalog_js.push(','); }
        catalog_js.push_str(&format!(
            r#"{{"idx":{},"distro":"{}","name":"{}","url":"{}","ct":"{}","needs_auth":{},"codename":"{}","components":"{}","architectures":"{}"}}"#,
            i, kr.distro, kr.name, kr.url, kr.content_type, kr.needs_auth,
            kr.codename.unwrap_or(""), kr.components.unwrap_or(""), kr.architectures.unwrap_or("")
        ));
    }
    catalog_js.push(']');

    let mut rows = String::new();
    for repo in &repos {
        let state_badge = match repo.sync_state {
            RepoSyncState::Synced => r#"<span class="badge badge-green">Synced</span>"#,
            RepoSyncState::Syncing => r#"<span class="badge badge-yellow">Syncing</span>"#,
            RepoSyncState::Failed => r#"<span class="badge badge-red">Failed</span>"#,
            RepoSyncState::NotSynced => r#"<span class="badge badge-dim">Not Synced</span>"#,
        };
        let enabled_badge = if repo.enabled {
            r#"<span class="badge badge-green">Enabled</span>"#
        } else {
            r#"<span class="badge badge-dim">Disabled</span>"#
        };
        let last = repo.last_sync.as_deref()
            .map(relative_time)
            .unwrap_or_else(|| "never".to_string());
        let url_display = if repo.url.len() > 50 {
            format!("{}...", &repo.url[..47])
        } else {
            repo.url.clone()
        };
        let type_badge = if repo.content_type == "deb" {
            r#"<span class="badge badge-cyan">deb</span>"#
        } else {
            r#"<span class="badge badge-dim">yum</span>"#
        };
        let auth_icon = if repo.username.is_some() || repo.ssl_client_cert.is_some() {
            r#" <span title="Authenticated">&#x1f512;</span>"#
        } else {
            ""
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{name}{auth_icon}</td>
                <td>{type_badge}</td>
                <td class="url-cell" title="{url}">{url_display}</td>
                <td>{enabled_badge}</td>
                <td>{state_badge}</td>
                <td>{pkgs}</td>
                <td>{errata}</td>
                <td>{last}</td>
                <td>
                    <button class="sm" hx-post="/ui/repos/{id}/toggle" hx-swap="none"
                            hx-on::after-request="location.reload()">{toggle_label}</button>
                    <button class="sm" hx-post="/api/v1/repos/{id}/sync" hx-swap="none"
                            hx-on::after-request="location.reload()">Sync</button>
                    <button class="sm danger" hx-post="/ui/repos/{id}/delete" hx-swap="none"
                            hx-confirm="Delete repository '{name}'?"
                            hx-on::after-request="location.reload()">Delete</button>
                </td>
            </tr>"#,
            id = repo.id, name = repo.name,
            auth_icon = auth_icon,
            type_badge = type_badge,
            url = repo.url, url_display = url_display,
            enabled_badge = enabled_badge,
            pkgs = repo.package_count,
            errata = repo.errata_count,
            last = last,
            toggle_label = if repo.enabled { "Disable" } else { "Enable" },
        ));
    }

    let empty = if repos.is_empty() {
        r#"<div class="empty-state"><p>No repositories yet. Add repos from the catalog or create a custom one.</p></div>"#
    } else { "" };

    let content = format!(
        r##"<div class="toolbar">
    <h1>Repositories</h1>
    <div class="actions">
        <details class="create-form">
            <summary>+ From Catalog</summary>
            <div class="card">
                <form id="catalog-form" hx-post="/ui/repos/create-batch" hx-swap="none"
                      hx-on::after-request="if(event.detail.successful) location.reload()">
                    <input type="hidden" name="product_id" value="{first_product_id}">
                    <div class="form-row">
                        <div class="form-group">
                            <label>Distribution</label>
                            <select id="distro-select" onchange="updateCatalog()">
                                <option value="">-- Select --</option>
                                {distro_options}
                            </select>
                        </div>
                    </div>
                    <div id="auth-fields" class="form-row" style="display:none">
                        <div class="form-group">
                            <label>SSL Client Cert (PEM path)</label>
                            <input name="ssl_client_cert" placeholder="/data/stormstar/certs/entitlement.pem">
                        </div>
                        <div class="form-group">
                            <label>SSL Client Key (PEM path)</label>
                            <input name="ssl_client_key" placeholder="/data/stormstar/certs/entitlement-key.pem">
                        </div>
                    </div>
                    <div id="repo-checkboxes" style="margin:0.75rem 0"></div>
                    <div id="catalog-submit" style="display:none">
                        <button type="submit">Add Selected</button>
                    </div>
                </form>
            </div>
        </details>
        <details class="create-form">
            <summary>+ Custom</summary>
            <div class="card">
                <form hx-post="/ui/repos/create" hx-swap="none"
                      hx-on::after-request="if(event.detail.successful) location.reload()">
                    <div class="form-row">
                        <div class="form-group">
                            <label>Product</label>
                            <select name="product_id" required>{product_options}</select>
                        </div>
                        <div class="form-group">
                            <label>Type</label>
                            <select name="content_type" onchange="document.getElementById('deb-fields').style.display=this.value==='deb'?'flex':'none'">
                                <option value="yum">YUM (RPM)</option>
                                <option value="deb">APT (Deb)</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Name</label>
                            <input name="name" placeholder="e.g. CentOS Base" required>
                        </div>
                        <div class="form-group" style="flex:2">
                            <label>URL</label>
                            <input name="url" placeholder="https://mirror.centos.org/centos/8/BaseOS/x86_64/os/" required>
                        </div>
                    </div>
                    <div id="deb-fields" class="form-row" style="display:none">
                        <div class="form-group">
                            <label>Codename</label>
                            <input name="codename" placeholder="e.g. jammy, bookworm">
                        </div>
                        <div class="form-group">
                            <label>Components</label>
                            <input name="components" placeholder="e.g. main,restricted,universe">
                        </div>
                        <div class="form-group">
                            <label>Architectures</label>
                            <input name="architectures" placeholder="e.g. amd64,arm64">
                        </div>
                    </div>
                    <div class="form-row">
                        <div class="form-group">
                            <label>Username (optional)</label>
                            <input name="username" placeholder="Basic auth username">
                        </div>
                        <div class="form-group">
                            <label>Password (optional)</label>
                            <input name="password" type="password" placeholder="Basic auth password">
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
</div>
{empty}
<div class="card">
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Type</th>
                <th>URL</th>
                <th>Enabled</th>
                <th>Sync Status</th>
                <th>Packages</th>
                <th>Errata</th>
                <th>Last Sync</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>{rows}</tbody>
    </table>
</div>
<script>
var CATALOG = {catalog_js};
function updateCatalog() {{
    var distro = document.getElementById('distro-select').value;
    var box = document.getElementById('repo-checkboxes');
    var auth = document.getElementById('auth-fields');
    var submit = document.getElementById('catalog-submit');
    box.innerHTML = '';
    if (!distro) {{ auth.style.display='none'; submit.style.display='none'; return; }}
    var needs_auth = false;
    var html = '';
    CATALOG.forEach(function(r) {{
        if (r.distro === distro) {{
            if (r.needs_auth) needs_auth = true;
            html += '<label style="display:block;margin:0.3rem 0;cursor:pointer">';
            html += '<input type="checkbox" name="repos" value="' + r.idx + '" checked style="width:auto;margin-right:0.5rem">';
            html += r.name + ' <span style="color:var(--fg-dim);font-size:0.8rem">(' + r.url.substring(0,60) + '...)</span>';
            html += '</label>';
        }}
    }});
    box.innerHTML = html;
    auth.style.display = needs_auth ? 'flex' : 'none';
    submit.style.display = html ? 'block' : 'none';
    // Set hidden distro field
    var existing = document.querySelector('input[name="distro"]');
    if (!existing) {{
        var inp = document.createElement('input');
        inp.type = 'hidden'; inp.name = 'distro'; inp.value = distro;
        document.getElementById('catalog-form').appendChild(inp);
    }} else {{
        existing.value = distro;
    }}
}}
</script>"##,
        first_product_id = products.first().map(|p| p.id.as_str()).unwrap_or(""),
        distro_options = distro_options,
        product_options = product_options,
        catalog_js = catalog_js,
        rows = rows,
        empty = empty,
    );

    Html(style::layout("Repositories", "repos", &content))
}

// ── Handlers ────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<WebState>,
    Form(body): Form<CreateRepoForm>,
) -> StatusCode {
    let mut repo = if body.content_type == "deb" {
        Repository::new_deb(
            &body.product_id,
            &body.name,
            &body.url,
            body.codename.as_deref().unwrap_or("stable"),
            body.components.as_deref().unwrap_or("main"),
            body.architectures.as_deref().unwrap_or("amd64"),
        )
    } else {
        Repository::new(&body.product_id, &body.name, &body.url)
    };
    // Set auth fields if provided
    if let Some(u) = &body.username {
        if !u.is_empty() { repo.username = Some(u.clone()); }
    }
    if let Some(p) = &body.password {
        if !p.is_empty() { repo.password = Some(p.clone()); }
    }

    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if rw.insert(repo).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if rw.commit().is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn create_batch(
    State(state): State<WebState>,
    Form(body): Form<BatchCreateForm>,
) -> StatusCode {
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    for idx_str in &body.repos {
        let idx: usize = match idx_str.parse() {
            Ok(i) => i,
            Err(_) => continue,
        };
        if idx >= KNOWN_REPOS.len() { continue; }
        let kr = &KNOWN_REPOS[idx];

        let mut repo = if kr.content_type == "deb" {
            Repository::new_deb(
                &body.product_id,
                kr.name,
                kr.url,
                kr.codename.unwrap_or("stable"),
                kr.components.unwrap_or("main"),
                kr.architectures.unwrap_or("amd64"),
            )
        } else {
            Repository::new(&body.product_id, kr.name, kr.url)
        };

        // Set auth fields for RHEL CDN repos
        if kr.needs_auth {
            if let Some(cert) = &body.ssl_client_cert {
                if !cert.is_empty() { repo.ssl_client_cert = Some(cert.clone()); }
            }
            if let Some(key) = &body.ssl_client_key {
                if !key.is_empty() { repo.ssl_client_key = Some(key.clone()); }
            }
        }

        let _ = rw.insert(repo);
    }

    if rw.commit().is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

pub async fn toggle(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> StatusCode {
    let rw = match state.db.rw_transaction() {
        Ok(rw) => rw,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let old: Repository = match rw.get().primary(id) {
        Ok(Some(item)) => item,
        _ => return StatusCode::NOT_FOUND,
    };
    let mut updated = old.clone();
    updated.enabled = !old.enabled;
    if rw.update(old, updated).is_err() {
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
    let item: Repository = match rw.get().primary(id) {
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
