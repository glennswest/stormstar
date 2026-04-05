//! Host registration and inventory management.

use std::sync::Arc;

use anyhow::Result;
use native_db::Database;
use uuid::Uuid;

use crate::db::models::*;

/// Register a host using an activation key.
pub fn register_host(
    db: &Arc<Database<'static>>,
    activation_key_str: &str,
    hostname: &str,
    arch: &str,
    os: &str,
    facts: Vec<HostFact>,
) -> Result<Host> {
    let rw = db.rw_transaction()?;

    // Find the activation key
    let all_keys: Vec<ActivationKey> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let old_key = all_keys.into_iter()
        .find(|k| k.key == activation_key_str)
        .ok_or_else(|| anyhow::anyhow!("invalid activation key"))?;

    // Check usage limit
    if let Some(max) = old_key.max_hosts {
        if old_key.usage_count >= max {
            return Err(anyhow::anyhow!("activation key usage limit reached"));
        }
    }

    let host = Host {
        id: Uuid::new_v4().to_string(),
        org_id: old_key.org_id.clone(),
        hostname: hostname.to_string(),
        arch: arch.to_string(),
        os: os.to_string(),
        env_id: Some(old_key.env_id.clone()),
        cv_id: Some(old_key.cv_id.clone()),
        activation_key_id: Some(old_key.id.clone()),
        facts,
        installed_packages: Vec::new(),
        applicable_errata: Vec::new(),
        last_checkin: Some(chrono::Utc::now().to_rfc3339()),
        registered_at: chrono::Utc::now().to_rfc3339(),
    };

    // Increment usage
    let mut updated_key = old_key.clone();
    updated_key.usage_count += 1;

    rw.insert(host.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.update(old_key, updated_key).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Registered host '{}' with activation key", hostname);
    Ok(host)
}

/// Update installed packages for a host and compute applicable errata.
pub fn update_packages(
    db: &Arc<Database<'static>>,
    host_id: &str,
    installed_packages: Vec<String>,
) -> Result<Host> {
    let rw = db.rw_transaction()?;

    let old_host: Host = rw.get().primary(host_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("host not found: {}", host_id))?;

    // Compute applicable errata based on installed packages
    let all_errata: Vec<Erratum> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let applicable: Vec<String> = all_errata.iter()
        .filter(|e| {
            e.package_names.iter().any(|pn| {
                installed_packages.iter().any(|ip| ip.starts_with(pn))
            })
        })
        .map(|e| e.advisory_id.clone())
        .collect();

    let mut updated = old_host.clone();
    updated.installed_packages = installed_packages;
    updated.applicable_errata = applicable;
    updated.last_checkin = Some(chrono::Utc::now().to_rfc3339());

    rw.update(old_host, updated.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(updated)
}

/// Record a host checkin.
pub fn checkin(db: &Arc<Database<'static>>, host_id: &str) -> Result<Host> {
    let rw = db.rw_transaction()?;

    let old: Host = rw.get().primary(host_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("host not found: {}", host_id))?;

    let mut updated = old.clone();
    updated.last_checkin = Some(chrono::Utc::now().to_rfc3339());

    rw.update(old, updated.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(updated)
}
