//! TOML configuration parsing.

use serde::{Deserialize, Serialize};

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,

    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    #[serde(default)]
    pub tls: Option<TlsConfig>,

    #[serde(default)]
    pub organization: Option<String>,

    #[serde(default)]
    pub log_level: Option<String>,

    #[serde(default = "default_download_concurrency")]
    pub download_concurrency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}

fn default_listen() -> String {
    "0.0.0.0:8585".to_string()
}

fn default_data_dir() -> String {
    "/data/stormstar".to_string()
}

fn default_download_concurrency() -> usize {
    4
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            data_dir: default_data_dir(),
            tls: None,
            organization: None,
            log_level: None,
            download_concurrency: default_download_concurrency(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read config '{}': {e}", path))?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse config '{}': {e}", path))?;
        Ok(config)
    }

    /// Ensure data directories exist.
    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        let dirs = [
            &self.data_dir,
            &format!("{}/db", self.data_dir),
            &format!("{}/packages", self.data_dir),
            &format!("{}/repos", self.data_dir),
            &format!("{}/cv_versions", self.data_dir),
            &format!("{}/environments", self.data_dir),
        ];
        for dir in dirs {
            std::fs::create_dir_all(dir)
                .map_err(|e| anyhow::anyhow!("failed to create dir '{}': {e}", dir))?;
        }
        Ok(())
    }
}
