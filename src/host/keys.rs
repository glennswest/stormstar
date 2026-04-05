//! Activation key management.

use std::sync::Arc;

use anyhow::Result;
use native_db::Database;

use crate::db::models::*;

/// Create a new activation key.
pub fn create_key(
    db: &Arc<Database<'static>>,
    org_id: &str,
    name: &str,
    env_id: &str,
    cv_id: &str,
    max_hosts: Option<u64>,
) -> Result<ActivationKey> {
    let mut key = ActivationKey::new(org_id, name, env_id, cv_id);
    key.max_hosts = max_hosts;

    let rw = db.rw_transaction()?;
    rw.insert(key.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Created activation key '{}' (key: {})", name, key.key);
    Ok(key)
}

/// List all activation keys for an organization.
pub fn list_keys(db: &Arc<Database<'static>>, org_id: &str) -> Result<Vec<ActivationKey>> {
    let r = db.r_transaction()?;

    let all: Vec<ActivationKey> = r.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(all.into_iter().filter(|k| k.org_id == org_id).collect())
}

/// Get activation key usage stats.
pub fn key_usage(db: &Arc<Database<'static>>, key_id: &str) -> Result<(u64, Option<u64>)> {
    let r = db.r_transaction()?;

    let key: ActivationKey = r.get().primary(key_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("activation key not found: {}", key_id))?;

    Ok((key.usage_count, key.max_hosts))
}
