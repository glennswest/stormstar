//! APT (Debian/Ubuntu) repository metadata parsing and generation.
//!
//! Handles RFC822-style Packages files, Release files, and the
//! dists/pool URL layout used by APT repositories.

use std::io::Read;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

/// A parsed entry from a Packages file.
#[derive(Debug, Clone)]
pub struct DebPackage {
    pub package: String,
    pub version: String,
    pub architecture: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
    pub description: String,
    pub section: String,
    pub source: String,
}

/// Parsed Release file metadata.
#[derive(Debug, Clone)]
pub struct DebRelease {
    pub origin: String,
    pub label: String,
    pub suite: String,
    pub codename: String,
    pub architectures: Vec<String>,
    pub components: Vec<String>,
    pub sha256_entries: Vec<ReleaseChecksum>,
}

/// A checksum entry from a Release file (SHA256 section).
#[derive(Debug, Clone)]
pub struct ReleaseChecksum {
    pub hash: String,
    pub size: u64,
    pub filename: String,
}

/// Parse a Debian version string into (epoch, upstream, revision).
///
/// Format: `[epoch:]upstream[-revision]`
/// Examples:
///   "2:1.18.0-6ubuntu14" → ("2", "1.18.0", "6ubuntu14")
///   "1.2.3-1" → ("0", "1.2.3", "1")
///   "1.0" → ("0", "1.0", "")
pub fn parse_deb_version(version: &str) -> (String, String, String) {
    let (epoch, rest) = if let Some(pos) = version.find(':') {
        (version[..pos].to_string(), &version[pos + 1..])
    } else {
        ("0".to_string(), version)
    };

    let (upstream, revision) = if let Some(pos) = rest.rfind('-') {
        (rest[..pos].to_string(), rest[pos + 1..].to_string())
    } else {
        (rest.to_string(), String::new())
    };

    (epoch, upstream, revision)
}

/// Parse an RFC822-style Packages file into a list of DebPackage entries.
pub fn parse_packages(text: &str) -> Vec<DebPackage> {
    let mut packages = Vec::new();
    let mut current = new_deb_package();
    let mut last_field = String::new();

    for line in text.lines() {
        if line.is_empty() {
            // End of stanza
            if !current.package.is_empty() {
                packages.push(current);
            }
            current = new_deb_package();
            last_field.clear();
            continue;
        }

        // Continuation line (starts with space or tab)
        if line.starts_with(' ') || line.starts_with('\t') {
            if last_field == "Description" {
                current.description.push('\n');
                current.description.push_str(line.trim());
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            last_field = key.to_string();

            match key {
                "Package" => current.package = value.to_string(),
                "Version" => current.version = value.to_string(),
                "Architecture" => current.architecture = value.to_string(),
                "Filename" => current.filename = value.to_string(),
                "Size" => current.size = value.parse().unwrap_or(0),
                "SHA256" => current.sha256 = value.to_string(),
                "Description" => current.description = value.to_string(),
                "Section" => current.section = value.to_string(),
                "Source" => current.source = value.to_string(),
                _ => {}
            }
        }
    }

    // Don't forget the last stanza if file doesn't end with blank line
    if !current.package.is_empty() {
        packages.push(current);
    }

    packages
}

fn new_deb_package() -> DebPackage {
    DebPackage {
        package: String::new(),
        version: String::new(),
        architecture: String::new(),
        filename: String::new(),
        size: 0,
        sha256: String::new(),
        description: String::new(),
        section: String::new(),
        source: String::new(),
    }
}

/// Parse a Release file to extract metadata and checksums.
pub fn parse_release(text: &str) -> DebRelease {
    let mut release = DebRelease {
        origin: String::new(),
        label: String::new(),
        suite: String::new(),
        codename: String::new(),
        architectures: Vec::new(),
        components: Vec::new(),
        sha256_entries: Vec::new(),
    };

    let mut in_sha256 = false;

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        // Inside SHA256 block: lines start with a space
        if in_sha256 {
            if line.starts_with(' ') || line.starts_with('\t') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    release.sha256_entries.push(ReleaseChecksum {
                        hash: parts[0].to_string(),
                        size: parts[1].parse().unwrap_or(0),
                        filename: parts[2].to_string(),
                    });
                }
                continue;
            } else {
                in_sha256 = false;
            }
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "Origin" => release.origin = value.to_string(),
                "Label" => release.label = value.to_string(),
                "Suite" => release.suite = value.to_string(),
                "Codename" => release.codename = value.to_string(),
                "Architectures" => {
                    release.architectures = value.split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                }
                "Components" => {
                    release.components = value.split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                }
                "SHA256" => {
                    in_sha256 = true;
                }
                _ => {}
            }
        }
    }

    release
}

/// Generate a Packages file from database Package records.
pub fn generate_packages(packages: &[crate::db::models::Package], component: &str) -> String {
    let mut out = String::new();

    for pkg in packages {
        let (epoch, upstream, revision) = parse_deb_version(&pkg.version);
        let deb_version = if epoch != "0" {
            format!("{}:{}", epoch, pkg.version)
        } else {
            pkg.version.clone()
        };

        let source = pkg.name.clone();
        let prefix = if pkg.name.starts_with("lib") && pkg.name.len() > 3 {
            format!("lib{}", &pkg.name[3..4])
        } else {
            pkg.name[..1].to_string()
        };

        let filename = if pkg.location_href.is_empty() {
            format!("pool/{}/{}/{}/{}", component, prefix, source, pkg.deb_filename())
        } else {
            pkg.location_href.clone()
        };

        out.push_str(&format!("Package: {}\n", pkg.name));
        out.push_str(&format!("Version: {}\n", deb_version));
        out.push_str(&format!("Architecture: {}\n", pkg.arch));
        out.push_str(&format!("Filename: {}\n", filename));
        out.push_str(&format!("Size: {}\n", pkg.size));
        if !pkg.sha256.is_empty() {
            out.push_str(&format!("SHA256: {}\n", pkg.sha256));
        }
        if !pkg.summary.is_empty() {
            out.push_str(&format!("Description: {}\n", pkg.summary));
        }
        // Reconstruct section and source from version parts
        if !upstream.is_empty() || !revision.is_empty() {
            out.push_str(&format!("Source: {}\n", source));
        }
        out.push('\n');
    }

    out
}

/// Generate a Release file.
pub fn generate_release(
    codename: &str,
    architectures: &[String],
    components: &[String],
    entries: &[(String, Vec<u8>)],  // (relative_path, content)
) -> String {
    let mut out = String::new();
    let now = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S UTC").to_string();

    out.push_str("Origin: StormStar\n");
    out.push_str("Label: StormStar\n");
    out.push_str(&format!("Suite: {}\n", codename));
    out.push_str(&format!("Codename: {}\n", codename));
    out.push_str(&format!("Date: {}\n", now));
    out.push_str(&format!("Architectures: {}\n", architectures.join(" ")));
    out.push_str(&format!("Components: {}\n", components.join(" ")));

    if !entries.is_empty() {
        out.push_str("SHA256:\n");
        for (path, content) in entries {
            let hash = hex::encode(Sha256::digest(content));
            out.push_str(&format!(" {} {} {}\n", hash, content.len(), path));
        }
    }

    out
}

/// Decompress gzipped Packages.gz data to plaintext.
pub fn decompress_packages_gz(data: &[u8]) -> Result<String> {
    let mut decoder = GzDecoder::new(data);
    let mut text = String::new();
    decoder.read_to_string(&mut text)
        .context("failed to decompress Packages.gz")?;
    Ok(text)
}
