//! Lifecycle environment promotion logic.
//!
//! Content view versions are promoted through a chain of environments:
//!   Library → Development → Testing → Production
//!
//! Each environment holds a reference to one content view version.
//! Promoting copies the version reference to the next environment.

use std::sync::Arc;

use anyhow::Result;
use native_db::Database;

use crate::db::models::*;

/// Promote a content view version to a lifecycle environment.
pub fn promote(
    db: &Arc<Database<'static>>,
    cv_id: &str,
    version_num: u32,
    env_id: &str,
) -> Result<LifecycleEnvironment> {
    let rw = db.rw_transaction()?;

    // Verify the content view exists
    let _cv: ContentView = rw.get().primary(cv_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("content view not found: {}", cv_id))?;

    // Find the specific version
    let all_versions: Vec<ContentViewVersion> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let version = all_versions.iter()
        .find(|v| v.cv_id == cv_id && v.version == version_num)
        .ok_or_else(|| anyhow::anyhow!("content view version {} not found", version_num))?;

    // Load the target environment
    let old_env: LifecycleEnvironment = rw.get().primary(env_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("environment not found: {}", env_id))?;

    // Validate promotion order: if the environment has a prior,
    // the prior must already have this or a newer CV version
    if let Some(ref prior_id) = old_env.prior_id {
        let prior: LifecycleEnvironment = rw.get().primary(prior_id.clone())?
            .ok_or_else(|| anyhow::anyhow!("prior environment not found"))?;

        if prior.cv_version_id.is_none() {
            return Err(anyhow::anyhow!(
                "cannot promote to '{}': prior environment '{}' has no content",
                old_env.name, prior.name
            ));
        }
    }

    // Update the environment with the new version
    let mut updated_env = old_env.clone();
    updated_env.cv_version_id = Some(version.id.clone());

    rw.update(old_env, updated_env.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Promoted CV version {} to environment '{}'",
        version_num, updated_env.name);

    Ok(updated_env)
}

/// Create the default lifecycle environment chain for an organization.
pub fn create_default_chain(
    db: &Arc<Database<'static>>,
    org_id: &str,
) -> Result<Vec<LifecycleEnvironment>> {
    let names = ["Library", "Development", "Testing", "Production"];
    let mut envs = Vec::new();
    let mut prior_id: Option<String> = None;

    let rw = db.rw_transaction()?;

    for (i, name) in names.iter().enumerate() {
        let env = LifecycleEnvironment::new(
            org_id,
            name,
            i as u32,
            prior_id.as_deref(),
        );

        // Link prior's successor
        if let Some(ref pid) = prior_id {
            let old_prior: LifecycleEnvironment = rw.get().primary(pid.clone())?
                .ok_or_else(|| anyhow::anyhow!("prior env missing during chain creation"))?;
            let mut updated_prior = old_prior.clone();
            updated_prior.successor_id = Some(env.id.clone());
            rw.update(old_prior, updated_prior).map_err(|e| anyhow::anyhow!("{}", e))?;
        }

        prior_id = Some(env.id.clone());
        rw.insert(env.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
        envs.push(env);
    }

    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Created default lifecycle chain for org {}", org_id);
    Ok(envs)
}
