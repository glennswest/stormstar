//! updateinfo.xml errata parser + standalone errata sync engine.

use std::sync::Arc;

use anyhow::{Context, Result};
use native_db::Database;
use quick_xml::Reader;
use quick_xml::events::Event;
use uuid::Uuid;

use crate::db::models::*;
use super::repodata;

/// Parsed erratum from updateinfo.xml.
#[derive(Debug, Clone)]
pub struct ParsedErratum {
    pub advisory_id: String,
    pub title: String,
    pub erratum_type: String,
    pub severity: String,
    pub description: String,
    pub issued: String,
    pub updated: String,
    pub cves: Vec<String>,
    pub package_names: Vec<String>,
}

/// Parse updateinfo.xml to extract errata.
pub fn parse_updateinfo(xml: &str) -> Result<Vec<ParsedErratum>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut errata = Vec::new();
    let mut current: Option<ParsedErratum> = None;
    let mut current_tag = String::new();
    let mut in_pkglist = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"update" => {
                        let mut etype = String::from("Bugfix");
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"type" {
                                etype = match String::from_utf8_lossy(&attr.value).as_ref() {
                                    "security" => "Security".to_string(),
                                    "bugfix" => "Bugfix".to_string(),
                                    "enhancement" => "Enhancement".to_string(),
                                    other => other.to_string(),
                                };
                            }
                        }
                        current = Some(ParsedErratum {
                            advisory_id: String::new(),
                            title: String::new(),
                            erratum_type: etype,
                            severity: "None".to_string(),
                            description: String::new(),
                            issued: String::new(),
                            updated: String::new(),
                            cves: Vec::new(),
                            package_names: Vec::new(),
                        });
                    }
                    b"id" | b"title" | b"severity" | b"description" if current.is_some() => {
                        current_tag = String::from_utf8_lossy(local.as_ref()).to_string();
                    }
                    b"pkglist" => in_pkglist = true,
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                if current.is_none() {
                    buf.clear();
                    continue;
                }
                let local = e.local_name();
                match local.as_ref() {
                    b"issued" => {
                        if let Some(ref mut er) = current {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"date" {
                                    er.issued = String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                    }
                    b"updated" => {
                        if let Some(ref mut er) = current {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"date" {
                                    er.updated = String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                    }
                    b"reference" => {
                        if let Some(ref mut er) = current {
                            let mut is_cve = false;
                            let mut ref_id = String::new();
                            for attr in e.attributes().flatten() {
                                match attr.key.local_name().as_ref() {
                                    b"type" => {
                                        is_cve = String::from_utf8_lossy(&attr.value) == "cve";
                                    }
                                    b"id" => {
                                        ref_id = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    _ => {}
                                }
                            }
                            if is_cve && !ref_id.is_empty() {
                                er.cves.push(ref_id);
                            }
                        }
                    }
                    b"package" if in_pkglist => {
                        if let Some(ref mut er) = current {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"name" {
                                    let name = String::from_utf8_lossy(&attr.value).to_string();
                                    if !er.package_names.contains(&name) {
                                        er.package_names.push(name);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut er) = current {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "id" => er.advisory_id = text,
                        "title" => er.title = text,
                        "severity" => er.severity = text,
                        "description" => er.description = text,
                        _ => {}
                    }
                    current_tag.clear();
                }
            }
            Ok(Event::End(e)) => {
                match e.local_name().as_ref() {
                    b"update" => {
                        if let Some(er) = current.take() {
                            errata.push(er);
                        }
                    }
                    b"pkglist" => in_pkglist = false,
                    _ => {}
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e).context("failed to parse updateinfo.xml"),
            _ => {}
        }
        buf.clear();
    }

    Ok(errata)
}

/// Sync errata for a single repository by re-fetching updateinfo.xml.
/// Deb repos have no errata — returns Ok(0) immediately.
pub async fn sync_errata(db: &Arc<Database<'static>>, repo_id: &str) -> Result<u64> {
    let repo = {
        let r = db.r_transaction()?;
        r.get().primary::<Repository>(repo_id.to_string())?
            .ok_or_else(|| anyhow::anyhow!("repository not found: {}", repo_id))?
    };

    if repo.content_type == "deb" {
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let base_url = repo.url.trim_end_matches('/');

    // Fetch repomd.xml to locate updateinfo entry
    let repomd_url = format!("{}/repodata/repomd.xml", base_url);
    let repomd_xml = client.get(&repomd_url).send().await?
        .error_for_status()
        .context("failed to fetch repomd.xml")?
        .text().await?;

    let entries = repodata::parse_repomd(&repomd_xml)?;

    let updateinfo_entry = entries.iter()
        .find(|e| e.data_type == "updateinfo")
        .ok_or_else(|| anyhow::anyhow!("no updateinfo in repomd.xml for '{}'", repo.name))?;

    let updateinfo_url = format!("{}/{}", base_url, updateinfo_entry.location);
    tracing::info!("Fetching errata from {}", updateinfo_url);

    let updateinfo_gz = client.get(&updateinfo_url).send().await?
        .error_for_status()
        .context("failed to fetch updateinfo.xml")?
        .bytes().await?;

    let updateinfo_xml = repodata::decompress_gz(&updateinfo_gz)?;
    let parsed_errata = parse_updateinfo(&updateinfo_xml)?;

    // Clear old errata for this repo, insert new
    let rw = db.rw_transaction()?;
    let existing: Vec<Erratum> = rw.scan().primary()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .all()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for er in existing.iter().filter(|e| e.repo_id == repo.id) {
        rw.remove(er.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    let count = parsed_errata.len() as u64;
    for pe in &parsed_errata {
        let erratum = Erratum {
            id: Uuid::new_v4().to_string(),
            advisory_id: pe.advisory_id.clone(),
            repo_id: repo.id.clone(),
            title: pe.title.clone(),
            erratum_type: match pe.erratum_type.as_str() {
                "Security" => ErratumType::Security,
                "Enhancement" => ErratumType::Enhancement,
                _ => ErratumType::Bugfix,
            },
            severity: match pe.severity.as_str() {
                "Critical" => ErratumSeverity::Critical,
                "Important" => ErratumSeverity::Important,
                "Moderate" => ErratumSeverity::Moderate,
                "Low" => ErratumSeverity::Low,
                _ => ErratumSeverity::None,
            },
            description: pe.description.clone(),
            issued: pe.issued.clone(),
            updated: pe.updated.clone(),
            cves: pe.cves.clone(),
            package_names: pe.package_names.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        rw.insert(erratum).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    // Update repo errata count
    let old = rw.get().primary::<Repository>(repo.id.clone())
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .ok_or_else(|| anyhow::anyhow!("repository vanished during errata sync"))?;
    let mut updated = old.clone();
    updated.errata_count = count;
    rw.update(old, updated).map_err(|e| anyhow::anyhow!("{}", e))?;

    rw.commit().map_err(|e| anyhow::anyhow!("{}", e))?;

    tracing::info!("Synced {} errata for '{}'", count, repo.name);
    Ok(count)
}

/// Sync errata across all synced repositories.
pub async fn sync_all_errata(db: &Arc<Database<'static>>) -> Result<u64> {
    let repos = {
        let r = db.r_transaction()?;
        let all: Vec<Repository> = r.scan().primary()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .all()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        all.into_iter()
            .filter(|r| r.sync_state == RepoSyncState::Synced && r.content_type != "deb")
            .collect::<Vec<_>>()
    };

    tracing::info!("Syncing errata for {} synced repositories", repos.len());
    let mut total = 0u64;
    for repo in &repos {
        match sync_errata(db, &repo.id).await {
            Ok(count) => total += count,
            Err(e) => tracing::warn!("Errata sync skipped for '{}': {}", repo.name, e),
        }
    }

    tracing::info!("Errata sync complete: {} total errata across all repos", total);
    Ok(total)
}
