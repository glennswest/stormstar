//! StormStar — Lightweight RPM content management.

use std::sync::Arc;
use clap::Parser;

use stormstar::config::Config;
use stormstar::db;

#[derive(Parser)]
#[command(name = "stormstar", version = "0.1.0", about = "Lightweight RPM content management")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/stormstar/stormstar.toml")]
    config: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Start the StormStar server
    Serve,

    /// Repository management
    Repo {
        #[command(subcommand)]
        action: stormstar::cli::RepoAction,
    },

    /// Content view management
    Cv {
        #[command(subcommand)]
        action: stormstar::cli::CvAction,
    },

    /// Lifecycle environment management
    Env {
        #[command(subcommand)]
        action: stormstar::cli::EnvAction,
    },

    /// Host management
    Host {
        #[command(subcommand)]
        action: stormstar::cli::HostAction,
    },

    /// Activation key management
    Key {
        #[command(subcommand)]
        action: stormstar::cli::KeyAction,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load config (use defaults if file doesn't exist for serve)
    let config = if std::path::Path::new(&cli.config).exists() {
        Config::from_file(&cli.config)?
    } else {
        match &cli.command {
            Command::Serve => {
                tracing::info!("No config file found, using defaults");
                Config::default()
            }
            _ => Config::default(),
        }
    };

    match cli.command {
        Command::Serve => {
            // Init logging
            let filter = config.log_level.as_deref().unwrap_or("info");
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .init();

            tracing::info!("StormStar v0.1.0 starting");

            // Ensure data directories
            config.ensure_dirs()?;

            // Open database
            let db_path = format!("{}/db/stormstar.db", config.data_dir);
            let database = db::open_db(&db_path)?;
            tracing::info!("Database opened at {}", db_path);

            let database = Arc::new(database);
            let config = Arc::new(config);

            // Build API router
            let state = stormstar::api::AppState {
                db: database,
                config: config.clone(),
            };
            let app = stormstar::api::router(state);

            // Bind and serve
            let listener = tokio::net::TcpListener::bind(config.listen.as_str()).await?;
            tracing::info!("Listening on {}", config.listen);

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;

            tracing::info!("Shutdown complete");
        }
        Command::Repo { action } => stormstar::cli::handle_repo(&config, action).await?,
        Command::Cv { action } => stormstar::cli::handle_cv(&config, action).await?,
        Command::Env { action } => stormstar::cli::handle_env(&config, action).await?,
        Command::Host { action } => stormstar::cli::handle_host(&config, action).await?,
        Command::Key { action } => stormstar::cli::handle_key(&config, action).await?,
    }

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");
    tracing::info!("Received Ctrl+C, shutting down...");
}
