//! Content view engine — compose, filter, publish.
//!
//! A content view selects repos and applies filters to create a curated
//! set of packages. Publishing creates a versioned snapshot.

use std::sync::Arc;

use anyhow::Result;
use native_db::Database;
use uuid::Uuid;

use crate::db::models::*;

/// Compose a content view: gather packages from all associated repos,
/// apply include/exclude filters, return the filtered package list.
pub fn compose(db: &Arc<Database<'static>>, cv_id: &str) -> Result<Vec<Package>> {
    let r = db.r_transaction()?;

    let cv: ContentView = r.get().primary(cv_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("content view not found: {}", cv_id))?;

    // Gather all packages from associated repos
    let all_packages: Vec<Package> = r.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut packages: Vec<Package> = all_packages.into_iter()
        .filter(|p| cv.repo_ids.contains(&p.repo_id))
        .collect();

    // Load and apply filters
    let all_filters: Vec<ContentViewFilter> = r.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let filters: Vec<&ContentViewFilter> = all_filters.iter()
        .filter(|f| cv.filter_ids.contains(&f.id))
        .collect();

    for filter in &filters {
        if filter.content_type != FilterContentType::Rpm {
            continue;
        }
        match filter.filter_type {
            FilterType::Include => {
                packages.retain(|p| matches_any_rule(p, &filter.rules));
            }
            FilterType::Exclude => {
                packages.retain(|p| !matches_any_rule(p, &filter.rules));
            }
        }
    }

    Ok(packages)
}

/// Check if a package matches any of the filter rules.
fn matches_any_rule(pkg: &Package, rules: &[FilterRule]) -> bool {
    rules.iter().any(|rule| matches_rule(pkg, rule))
}

/// Check if a package matches a single filter rule.
fn matches_rule(pkg: &Package, rule: &FilterRule) -> bool {
    let field_value = match rule.field.as_str() {
        "name" => &pkg.name,
        "arch" => &pkg.arch,
        "version" => &pkg.version,
        "release" => &pkg.release,
        "epoch" => &pkg.epoch,
        _ => return false,
    };

    match rule.operator.as_str() {
        "equals" => field_value == &rule.value,
        "matches" => {
            // Simple glob match: * at start/end
            if rule.value.starts_with('*') && rule.value.ends_with('*') {
                let pattern = &rule.value[1..rule.value.len() - 1];
                field_value.contains(pattern)
            } else if rule.value.starts_with('*') {
                field_value.ends_with(&rule.value[1..])
            } else if rule.value.ends_with('*') {
                field_value.starts_with(&rule.value[..rule.value.len() - 1])
            } else {
                field_value == &rule.value
            }
        }
        "contains" => field_value.contains(&rule.value),
        _ => false,
    }
}

/// Publish a content view: compose, snapshot, and create a new version.
pub fn publish(db: &Arc<Database<'static>>, cv_id: &str) -> Result<ContentViewVersion> {
    let packages = compose(db, cv_id)?;

    let rw = db.rw_transaction()?;

    let old_cv: ContentView = rw.get().primary(cv_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("content view not found"))?;

    let new_version_num = old_cv.latest_version + 1;

    // Count errata from associated repos
    let all_errata: Vec<Erratum> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let errata_count = all_errata.iter()
        .filter(|e| old_cv.repo_ids.contains(&e.repo_id))
        .count() as u64;

    let version = ContentViewVersion {
        id: Uuid::new_v4().to_string(),
        cv_id: cv_id.to_string(),
        version: new_version_num,
        package_count: packages.len() as u64,
        errata_count,
        repo_ids: old_cv.repo_ids.clone(),
        published_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut updated_cv = old_cv.clone();
    updated_cv.latest_version = new_version_num;

    rw.insert(version.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.update(old_cv, updated_cv).map_err(|e| anyhow::anyhow!("{}", e))?;
    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Published content view version {} ({} packages, {} errata)",
        new_version_num, packages.len(), errata_count);

    Ok(version)
}
