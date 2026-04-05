//! updateinfo.xml errata parser.

use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;

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
