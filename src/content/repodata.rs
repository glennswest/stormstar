//! Repodata parsing and generation — repomd.xml, primary.xml.gz, updateinfo.xml.gz.

use std::io::Read;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::db::models::Package;

/// Location of a repodata file within the repository.
#[derive(Debug, Clone)]
pub struct RepomdEntry {
    pub data_type: String,
    pub location: String,
    pub checksum: Option<String>,
    pub size: Option<u64>,
}

/// Parsed package metadata from primary.xml.
#[derive(Debug, Clone)]
pub struct PrimaryPackage {
    pub name: String,
    pub arch: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub summary: String,
    pub sha256: String,
    pub size: u64,
    pub location_href: String,
}

/// Parse repomd.xml to extract data entries (primary, updateinfo, etc.).
pub fn parse_repomd(xml: &str) -> Result<Vec<RepomdEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut entries = Vec::new();
    let mut current: Option<RepomdEntry> = None;
    let mut in_checksum = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"data" => {
                        let mut dtype = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"type" {
                                dtype = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        current = Some(RepomdEntry {
                            data_type: dtype,
                            location: String::new(),
                            checksum: None,
                            size: None,
                        });
                    }
                    b"checksum" if current.is_some() => {
                        in_checksum = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let local = e.local_name();
                if local.as_ref() == b"location" {
                    if let Some(ref mut entry) = current {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"href" {
                                entry.location = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_checksum {
                    if let Some(ref mut entry) = current {
                        entry.checksum = Some(e.unescape().unwrap_or_default().to_string());
                    }
                    in_checksum = false;
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"data" {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e).context("failed to parse repomd.xml"),
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

/// Decompress gzipped data.
pub fn decompress_gz(data: &[u8]) -> Result<String> {
    let mut decoder = GzDecoder::new(data);
    let mut output = String::new();
    decoder.read_to_string(&mut output)
        .context("failed to decompress gzip data")?;
    Ok(output)
}

/// Parse primary.xml to extract package metadata.
pub fn parse_primary(xml: &str) -> Result<Vec<PrimaryPackage>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut packages = Vec::new();
    let mut current: Option<PrimaryPackage> = None;
    let mut current_tag = String::new();
    let mut in_checksum = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"package" => {
                        current = Some(PrimaryPackage {
                            name: String::new(),
                            arch: String::new(),
                            epoch: "0".to_string(),
                            version: String::new(),
                            release: String::new(),
                            summary: String::new(),
                            sha256: String::new(),
                            size: 0,
                            location_href: String::new(),
                        });
                    }
                    b"name" | b"arch" | b"summary" if current.is_some() => {
                        current_tag = String::from_utf8_lossy(local.as_ref()).to_string();
                    }
                    b"checksum" if current.is_some() => {
                        in_checksum = true;
                    }
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
                    b"version" => {
                        if let Some(ref mut pkg) = current {
                            for attr in e.attributes().flatten() {
                                match attr.key.local_name().as_ref() {
                                    b"epoch" => {
                                        pkg.epoch = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    b"ver" => {
                                        pkg.version = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    b"rel" => {
                                        pkg.release = String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    b"size" => {
                        if let Some(ref mut pkg) = current {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"package" {
                                    if let Ok(s) = String::from_utf8_lossy(&attr.value).parse::<u64>() {
                                        pkg.size = s;
                                    }
                                }
                            }
                        }
                    }
                    b"location" => {
                        if let Some(ref mut pkg) = current {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"href" {
                                    pkg.location_href = String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut pkg) = current {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if in_checksum {
                        pkg.sha256 = text;
                        in_checksum = false;
                    } else {
                        match current_tag.as_str() {
                            "name" => pkg.name = text,
                            "arch" => pkg.arch = text,
                            "summary" => pkg.summary = text,
                            _ => {}
                        }
                        current_tag.clear();
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"package" {
                    if let Some(pkg) = current.take() {
                        packages.push(pkg);
                    }
                }
                current_tag.clear();
                in_checksum = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e).context("failed to parse primary.xml"),
            _ => {}
        }
        buf.clear();
    }

    Ok(packages)
}

// ── Repodata generation ──────────────────────────────────────────────

/// Generate repomd.xml pointing to the primary.xml.gz we serve.
pub fn generate_repomd(packages: &[Package]) -> String {
    use sha2::{Sha256, Digest};
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let primary_xml = generate_primary(packages);

    // Compute checksum and size of the gzipped primary
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(primary_xml.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let checksum = hex::encode(hasher.finalize());
    let size = compressed.len();

    // Open checksum (uncompressed)
    let mut open_hasher = Sha256::new();
    open_hasher.update(primary_xml.as_bytes());
    let open_checksum = hex::encode(open_hasher.finalize());
    let open_size = primary_xml.len();

    let ts = chrono::Utc::now().timestamp();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<repomd xmlns="http://linux.duke.edu/metadata/repo">
  <revision>{ts}</revision>
  <data type="primary">
    <checksum type="sha256">{checksum}</checksum>
    <open-checksum type="sha256">{open_checksum}</open-checksum>
    <location href="repodata/primary.xml.gz"/>
    <timestamp>{ts}</timestamp>
    <size>{size}</size>
    <open-size>{open_size}</open-size>
  </data>
</repomd>
"#
    )
}

/// Generate primary.xml from a list of packages in the database.
pub fn generate_primary(packages: &[Package]) -> String {
    use quick_xml::Writer;
    use quick_xml::events::{BytesStart, BytesText, BytesEnd, BytesDecl};

    let mut writer = Writer::new(Vec::new());

    // XML declaration
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None))).unwrap();

    // Root element
    let mut root = BytesStart::new("metadata");
    root.push_attribute(("xmlns", "http://linux.duke.edu/metadata/common"));
    root.push_attribute(("xmlns:rpm", "http://linux.duke.edu/metadata/rpm"));
    root.push_attribute(("packages", packages.len().to_string().as_str()));
    writer.write_event(Event::Start(root)).unwrap();

    for pkg in packages {
        // <package type="rpm">
        let mut pkg_start = BytesStart::new("package");
        pkg_start.push_attribute(("type", "rpm"));
        writer.write_event(Event::Start(pkg_start)).unwrap();

        // <name>
        writer.write_event(Event::Start(BytesStart::new("name"))).unwrap();
        writer.write_event(Event::Text(BytesText::new(&pkg.name))).unwrap();
        writer.write_event(Event::End(BytesEnd::new("name"))).unwrap();

        // <arch>
        writer.write_event(Event::Start(BytesStart::new("arch"))).unwrap();
        writer.write_event(Event::Text(BytesText::new(&pkg.arch))).unwrap();
        writer.write_event(Event::End(BytesEnd::new("arch"))).unwrap();

        // <version epoch="E" ver="V" rel="R"/>
        let mut ver = BytesStart::new("version");
        ver.push_attribute(("epoch", pkg.epoch.as_str()));
        ver.push_attribute(("ver", pkg.version.as_str()));
        ver.push_attribute(("rel", pkg.release.as_str()));
        writer.write_event(Event::Empty(ver)).unwrap();

        // <checksum type="sha256" pkgid="YES">HASH</checksum>
        let mut cksum = BytesStart::new("checksum");
        cksum.push_attribute(("type", "sha256"));
        cksum.push_attribute(("pkgid", "YES"));
        writer.write_event(Event::Start(cksum)).unwrap();
        writer.write_event(Event::Text(BytesText::new(&pkg.sha256))).unwrap();
        writer.write_event(Event::End(BytesEnd::new("checksum"))).unwrap();

        // <summary>
        writer.write_event(Event::Start(BytesStart::new("summary"))).unwrap();
        writer.write_event(Event::Text(BytesText::new(&pkg.summary))).unwrap();
        writer.write_event(Event::End(BytesEnd::new("summary"))).unwrap();

        // <size package="N"/>
        let mut size = BytesStart::new("size");
        size.push_attribute(("package", pkg.size.to_string().as_str()));
        writer.write_event(Event::Empty(size)).unwrap();

        // <location href="Packages/X/name-ver-rel.arch.rpm"/>
        let first_letter = pkg.name.chars().next().unwrap_or('_');
        let href = format!("Packages/{}/{}", first_letter, pkg.filename());
        let mut loc = BytesStart::new("location");
        loc.push_attribute(("href", href.as_str()));
        writer.write_event(Event::Empty(loc)).unwrap();

        // </package>
        writer.write_event(Event::End(BytesEnd::new("package"))).unwrap();
    }

    // </metadata>
    writer.write_event(Event::End(BytesEnd::new("metadata"))).unwrap();

    String::from_utf8(writer.into_inner()).unwrap()
}
