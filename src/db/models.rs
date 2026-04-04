//! Database entity models — 13 native_db entities.

use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Organization ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct Organization {
    #[primary_key]
    pub id: String,       // UUID as string
    #[secondary_key(unique)]
    pub name: String,
    pub label: String,
    pub description: String,
    pub created_at: String,
}

impl Organization {
    pub fn new(name: &str, label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            label: label.to_string(),
            description: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── Product ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct Product {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key(unique)]
    pub name: String,
    pub label: String,
    pub description: String,
    pub created_at: String,
}

impl Product {
    pub fn new(org_id: &str, name: &str, label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            label: label.to_string(),
            description: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── Repository ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepoSyncState {
    NotSynced,
    Syncing,
    Synced,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 3, version = 1)]
#[native_db]
pub struct Repository {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub product_id: String,
    #[secondary_key]
    pub name: String,
    pub label: String,
    pub url: String,
    pub content_type: String,   // "yum"
    pub arch: String,           // "x86_64", "noarch"
    pub sync_state: RepoSyncState,
    pub last_sync: Option<String>,
    pub package_count: u64,
    pub errata_count: u64,
    pub created_at: String,
}

impl Repository {
    pub fn new(product_id: &str, name: &str, url: &str) -> Self {
        let label = name.to_lowercase().replace(' ', "_");
        Self {
            id: Uuid::new_v4().to_string(),
            product_id: product_id.to_string(),
            name: name.to_string(),
            label,
            url: url.to_string(),
            content_type: "yum".to_string(),
            arch: "x86_64".to_string(),
            sync_state: RepoSyncState::NotSynced,
            last_sync: None,
            package_count: 0,
            errata_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── Package ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 4, version = 1)]
#[native_db]
pub struct Package {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub repo_id: String,
    #[secondary_key]
    pub name: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub summary: String,
    pub sha256: String,
    pub size: u64,
    pub location_href: String,
    pub created_at: String,
}

impl Package {
    pub fn nevra(&self) -> String {
        if self.epoch == "0" {
            format!("{}-{}-{}.{}", self.name, self.version, self.release, self.arch)
        } else {
            format!("{}:{}-{}-{}.{}", self.epoch, self.name, self.version, self.release, self.arch)
        }
    }

    pub fn filename(&self) -> String {
        format!("{}-{}-{}.{}.rpm", self.name, self.version, self.release, self.arch)
    }
}

// ── Erratum ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErratumType {
    Security,
    Bugfix,
    Enhancement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErratumSeverity {
    Critical,
    Important,
    Moderate,
    Low,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 5, version = 1)]
#[native_db]
pub struct Erratum {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub advisory_id: String,
    #[secondary_key]
    pub repo_id: String,
    pub title: String,
    pub erratum_type: ErratumType,
    pub severity: ErratumSeverity,
    pub description: String,
    pub issued: String,
    pub updated: String,
    pub cves: Vec<String>,
    pub package_names: Vec<String>,
    pub created_at: String,
}

// ── ContentView ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 6, version = 1)]
#[native_db]
pub struct ContentView {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key(unique)]
    pub name: String,
    pub label: String,
    pub description: String,
    pub repo_ids: Vec<String>,
    pub filter_ids: Vec<String>,
    pub latest_version: u32,
    pub created_at: String,
}

impl ContentView {
    pub fn new(org_id: &str, name: &str) -> Self {
        let label = name.to_lowercase().replace(' ', "_");
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            label,
            description: String::new(),
            repo_ids: Vec::new(),
            filter_ids: Vec::new(),
            latest_version: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── ContentViewVersion ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 7, version = 1)]
#[native_db]
pub struct ContentViewVersion {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub cv_id: String,
    pub version: u32,
    pub package_count: u64,
    pub errata_count: u64,
    pub repo_ids: Vec<String>,
    pub published_at: String,
}

// ── ContentViewFilter ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterType {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterContentType {
    Rpm,
    Erratum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 8, version = 1)]
#[native_db]
pub struct ContentViewFilter {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub cv_id: String,
    pub name: String,
    pub filter_type: FilterType,
    pub content_type: FilterContentType,
    pub rules: Vec<FilterRule>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterRule {
    pub field: String,      // "name", "advisory_type", "date"
    pub operator: String,   // "matches", "equals", "before", "after"
    pub value: String,
}

// ── LifecycleEnvironment ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 9, version = 1)]
#[native_db]
pub struct LifecycleEnvironment {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key(unique)]
    pub name: String,
    pub label: String,
    pub description: String,
    pub prior_id: Option<String>,
    pub successor_id: Option<String>,
    pub cv_version_id: Option<String>,
    pub position: u32,
    pub created_at: String,
}

impl LifecycleEnvironment {
    pub fn new(org_id: &str, name: &str, position: u32, prior_id: Option<&str>) -> Self {
        let label = name.to_lowercase().replace(' ', "_");
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            label,
            description: String::new(),
            prior_id: prior_id.map(|s| s.to_string()),
            successor_id: None,
            cv_version_id: None,
            position,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── Host ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 10, version = 1)]
#[native_db]
pub struct Host {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key]
    pub hostname: String,
    pub arch: String,
    pub os: String,
    pub env_id: Option<String>,
    pub cv_id: Option<String>,
    pub activation_key_id: Option<String>,
    pub facts: Vec<HostFact>,
    pub installed_packages: Vec<String>,
    pub applicable_errata: Vec<String>,
    pub last_checkin: Option<String>,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostFact {
    pub key: String,
    pub value: String,
}

// ── ActivationKey ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 11, version = 1)]
#[native_db]
pub struct ActivationKey {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key(unique)]
    pub name: String,
    pub key: String,
    pub env_id: String,
    pub cv_id: String,
    pub host_collection_ids: Vec<String>,
    pub max_hosts: Option<u64>,
    pub usage_count: u64,
    pub created_at: String,
}

impl ActivationKey {
    pub fn new(org_id: &str, name: &str, env_id: &str, cv_id: &str) -> Self {
        let key = format!("{}-{}", name, &Uuid::new_v4().to_string()[..8]);
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            key,
            env_id: env_id.to_string(),
            cv_id: cv_id.to_string(),
            host_collection_ids: Vec::new(),
            max_hosts: None,
            usage_count: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── SyncPlan ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 12, version = 1)]
#[native_db]
pub struct SyncPlan {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key]
    pub name: String,
    pub description: String,
    pub cron_expression: String,
    pub repo_ids: Vec<String>,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub created_at: String,
}

impl SyncPlan {
    pub fn new(org_id: &str, name: &str, cron_expression: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            description: String::new(),
            cron_expression: cron_expression.to_string(),
            repo_ids: Vec::new(),
            enabled: true,
            last_run: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ── HostCollection ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 13, version = 1)]
#[native_db]
pub struct HostCollection {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub org_id: String,
    #[secondary_key(unique)]
    pub name: String,
    pub description: String,
    pub host_ids: Vec<String>,
    pub max_hosts: Option<u64>,
    pub created_at: String,
}

impl HostCollection {
    pub fn new(org_id: &str, name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            org_id: org_id.to_string(),
            name: name.to_string(),
            description: String::new(),
            host_ids: Vec::new(),
            max_hosts: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
