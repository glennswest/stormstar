//! CLI subcommands — reqwest calls to own API.

use serde_json::Value;

use crate::config::Config;

fn base_url(config: &Config) -> String {
    let scheme = if config.tls.is_some() { "https" } else { "http" };
    format!("{}://{}/api/v1", scheme, config.listen)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build HTTP client")
}

async fn get_json(url: &str) -> anyhow::Result<Value> {
    let resp = client().get(url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }
    Ok(resp.json().await?)
}

async fn post_json(url: &str, body: &Value) -> anyhow::Result<Value> {
    let resp = client().post(url).json(body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }
    Ok(resp.json().await?)
}

async fn delete_req(url: &str) -> anyhow::Result<()> {
    let resp = client().delete(url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }
    Ok(())
}

// ── Repo commands ────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum RepoAction {
    /// List all repositories
    List,
    /// Create a new repository
    Create {
        #[arg(long)]
        product_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        url: String,
    },
    /// Trigger repository sync
    Sync { id: String },
    /// Delete a repository
    Delete { id: String },
}

pub async fn handle_repo(config: &Config, action: RepoAction) -> anyhow::Result<()> {
    let base = base_url(config);
    match action {
        RepoAction::List => {
            let v = get_json(&format!("{}/repos", base)).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        RepoAction::Create { product_id, name, url } => {
            let body = serde_json::json!({
                "product_id": product_id,
                "name": name,
                "url": url,
            });
            let v = post_json(&format!("{}/repos", base), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        RepoAction::Sync { id } => {
            let v = post_json(&format!("{}/repos/{}/sync", base, id), &serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        RepoAction::Delete { id } => {
            delete_req(&format!("{}/repos/{}", base, id)).await?;
            println!("Deleted repository {}", id);
        }
    }
    Ok(())
}

// ── Content View commands ────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum CvAction {
    /// List all content views
    List,
    /// Create a new content view
    Create {
        #[arg(long)]
        org_id: String,
        #[arg(long)]
        name: String,
    },
    /// Publish a content view version
    Publish { id: String },
    /// Promote a content view version to an environment
    Promote {
        id: String,
        #[arg(long)]
        version: u32,
        #[arg(long)]
        env: String,
    },
}

pub async fn handle_cv(config: &Config, action: CvAction) -> anyhow::Result<()> {
    let base = base_url(config);
    match action {
        CvAction::List => {
            let v = get_json(&format!("{}/content_views", base)).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        CvAction::Create { org_id, name } => {
            let body = serde_json::json!({ "org_id": org_id, "name": name });
            let v = post_json(&format!("{}/content_views", base), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        CvAction::Publish { id } => {
            let v = post_json(&format!("{}/content_views/{}/publish", base, id), &serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        CvAction::Promote { id, version, env } => {
            let body = serde_json::json!({ "version": version, "env_id": env });
            let v = post_json(&format!("{}/content_views/{}/promote", base, id), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

// ── Environment commands ─────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum EnvAction {
    /// List all lifecycle environments
    List,
    /// Create a new environment
    Create {
        #[arg(long)]
        org_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        prior: Option<String>,
    },
}

pub async fn handle_env(config: &Config, action: EnvAction) -> anyhow::Result<()> {
    let base = base_url(config);
    match action {
        EnvAction::List => {
            let v = get_json(&format!("{}/environments", base)).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        EnvAction::Create { org_id, name, prior } => {
            let mut body = serde_json::json!({ "org_id": org_id, "name": name });
            if let Some(p) = prior {
                body["prior_id"] = serde_json::json!(p);
            }
            let v = post_json(&format!("{}/environments", base), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

// ── Host commands ────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum HostAction {
    /// List all hosts
    List,
    /// Register a host with an activation key
    Register {
        #[arg(long)]
        key: String,
        #[arg(long)]
        hostname: String,
    },
    /// Show applicable errata for a host
    Errata { id: String },
}

pub async fn handle_host(config: &Config, action: HostAction) -> anyhow::Result<()> {
    let base = base_url(config);
    match action {
        HostAction::List => {
            let v = get_json(&format!("{}/hosts", base)).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        HostAction::Register { key, hostname } => {
            let body = serde_json::json!({
                "activation_key": key,
                "hostname": hostname,
            });
            let v = post_json(&format!("{}/hosts/register", base), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        HostAction::Errata { id } => {
            let v = get_json(&format!("{}/hosts/{}", base, id)).await?;
            if let Some(errata) = v.get("applicable_errata") {
                println!("{}", serde_json::to_string_pretty(errata)?);
            } else {
                println!("[]");
            }
        }
    }
    Ok(())
}

// ── Activation Key commands ──────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum KeyAction {
    /// List all activation keys
    List,
    /// Create a new activation key
    Create {
        #[arg(long)]
        org_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        env: String,
        #[arg(long)]
        cv: String,
    },
}

pub async fn handle_key(config: &Config, action: KeyAction) -> anyhow::Result<()> {
    let base = base_url(config);
    match action {
        KeyAction::List => {
            let v = get_json(&format!("{}/activation_keys", base)).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        KeyAction::Create { org_id, name, env, cv } => {
            let body = serde_json::json!({
                "org_id": org_id,
                "name": name,
                "env_id": env,
                "cv_id": cv,
            });
            let v = post_json(&format!("{}/activation_keys", base), &body).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}
