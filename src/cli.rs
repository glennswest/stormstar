//! CLI subcommands — reqwest calls to own API.

use crate::config::Config;

#[derive(clap::Subcommand)]
pub enum RepoAction {
    List,
    Create { #[arg(long)] name: String, #[arg(long)] url: String },
    Sync { id: String },
    Delete { id: String },
}

#[derive(clap::Subcommand)]
pub enum CvAction {
    List,
    Create { #[arg(long)] name: String },
    Publish { id: String },
    Promote { id: String, #[arg(long)] version: u32, #[arg(long)] env: String },
}

#[derive(clap::Subcommand)]
pub enum EnvAction {
    List,
    Create { #[arg(long)] name: String, #[arg(long)] prior: Option<String> },
}

#[derive(clap::Subcommand)]
pub enum HostAction {
    List,
    Register { #[arg(long)] key: String, #[arg(long)] hostname: String },
    Errata { id: String },
}

#[derive(clap::Subcommand)]
pub enum KeyAction {
    List,
    Create { #[arg(long)] name: String, #[arg(long)] env: String, #[arg(long)] cv: String },
}

pub async fn handle_repo(_config: &Config, _action: RepoAction) -> anyhow::Result<()> {
    // TODO: Phase 6
    println!("repo command not yet implemented");
    Ok(())
}

pub async fn handle_cv(_config: &Config, _action: CvAction) -> anyhow::Result<()> {
    println!("cv command not yet implemented");
    Ok(())
}

pub async fn handle_env(_config: &Config, _action: EnvAction) -> anyhow::Result<()> {
    println!("env command not yet implemented");
    Ok(())
}

pub async fn handle_host(_config: &Config, _action: HostAction) -> anyhow::Result<()> {
    println!("host command not yet implemented");
    Ok(())
}

pub async fn handle_key(_config: &Config, _action: KeyAction) -> anyhow::Result<()> {
    println!("key command not yet implemented");
    Ok(())
}
