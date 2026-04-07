//! Tests for repodata parsing and generation.

use stormstar::content::repodata;
use stormstar::db::models::Package;

const REPOMD_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<repomd xmlns="http://linux.duke.edu/metadata/repo">
  <revision>1234567890</revision>
  <data type="primary">
    <checksum type="sha256">abc123</checksum>
    <location href="repodata/abc-primary.xml.gz"/>
    <size>12345</size>
  </data>
  <data type="updateinfo">
    <checksum type="sha256">def456</checksum>
    <location href="repodata/def-updateinfo.xml.gz"/>
  </data>
  <data type="filelists">
    <checksum type="sha256">ghi789</checksum>
    <location href="repodata/ghi-filelists.xml.gz"/>
  </data>
</repomd>"#;

#[test]
fn test_parse_repomd() {
    let entries = repodata::parse_repomd(REPOMD_XML).unwrap();

    assert_eq!(entries.len(), 3);

    let primary = entries.iter().find(|e| e.data_type == "primary").unwrap();
    assert_eq!(primary.location, "repodata/abc-primary.xml.gz");
    assert_eq!(primary.checksum.as_deref(), Some("abc123"));

    let updateinfo = entries.iter().find(|e| e.data_type == "updateinfo").unwrap();
    assert_eq!(updateinfo.location, "repodata/def-updateinfo.xml.gz");
}

const PRIMARY_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="2">
  <package type="rpm">
    <name>bash</name>
    <arch>x86_64</arch>
    <version epoch="0" ver="5.1.8" rel="6.el9"/>
    <checksum type="sha256" pkgid="YES">aabbccdd</checksum>
    <summary>The GNU Bourne Again shell</summary>
    <size package="1884567" installed="2345678" archive="3456789"/>
    <location href="Packages/b/bash-5.1.8-6.el9.x86_64.rpm"/>
  </package>
  <package type="rpm">
    <name>vim-minimal</name>
    <arch>x86_64</arch>
    <version epoch="2" ver="9.0.1" rel="1.el9"/>
    <checksum type="sha256" pkgid="YES">eeff0011</checksum>
    <summary>A minimal version of the VIM editor</summary>
    <size package="700000"/>
    <location href="Packages/v/vim-minimal-9.0.1-1.el9.x86_64.rpm"/>
  </package>
</metadata>"#;

#[test]
fn test_parse_primary() {
    let packages = repodata::parse_primary(PRIMARY_XML).unwrap();

    assert_eq!(packages.len(), 2);

    let bash = &packages[0];
    assert_eq!(bash.name, "bash");
    assert_eq!(bash.arch, "x86_64");
    assert_eq!(bash.epoch, "0");
    assert_eq!(bash.version, "5.1.8");
    assert_eq!(bash.release, "6.el9");
    assert_eq!(bash.sha256, "aabbccdd");
    assert_eq!(bash.size, 1884567);
    assert_eq!(bash.location_href, "Packages/b/bash-5.1.8-6.el9.x86_64.rpm");
    assert_eq!(bash.summary, "The GNU Bourne Again shell");

    let vim = &packages[1];
    assert_eq!(vim.name, "vim-minimal");
    assert_eq!(vim.epoch, "2");
    assert_eq!(vim.version, "9.0.1");
    assert_eq!(vim.release, "1.el9");
}

#[test]
fn test_parse_primary_empty() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata xmlns="http://linux.duke.edu/metadata/common" packages="0">
</metadata>"#;
    let packages = repodata::parse_primary(xml).unwrap();
    assert!(packages.is_empty());
}

#[test]
fn test_decompress_gz() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let original = "hello world";
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let decompressed = repodata::decompress_gz(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn test_generate_repomd() {
    let packages = vec![
        Package {
            id: "pkg1".to_string(),
            repo_id: "repo1".to_string(),
            name: "bash".to_string(),
            epoch: "0".to_string(),
            version: "5.1.8".to_string(),
            release: "6.el9".to_string(),
            arch: "x86_64".to_string(),
            summary: "The GNU Bourne Again shell".to_string(),
            sha256: "aabbccdd".to_string(),
            size: 1884567,
            location_href: "Packages/b/bash-5.1.8-6.el9.x86_64.rpm".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    let xml = repodata::generate_repomd(&packages);
    assert!(xml.contains("<repomd"));
    assert!(xml.contains("type=\"primary\""));
    assert!(xml.contains("primary.xml.gz"));
    assert!(xml.contains("sha256"));
}

#[test]
fn test_generate_primary() {
    let packages = vec![
        Package {
            id: "pkg1".to_string(),
            repo_id: "repo1".to_string(),
            name: "bash".to_string(),
            epoch: "0".to_string(),
            version: "5.1.8".to_string(),
            release: "6.el9".to_string(),
            arch: "x86_64".to_string(),
            summary: "The GNU Bourne Again shell".to_string(),
            sha256: "aabbccdd".to_string(),
            size: 1884567,
            location_href: "Packages/b/bash-5.1.8-6.el9.x86_64.rpm".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    let xml = repodata::generate_primary(&packages);
    assert!(xml.contains("<metadata"));
    assert!(xml.contains("<name>bash</name>"));
    assert!(xml.contains("ver=\"5.1.8\""));
    assert!(xml.contains("rel=\"6.el9\""));
    assert!(xml.contains("aabbccdd"));
}

#[test]
fn test_roundtrip_primary() {
    // Generate → parse → verify
    let original = vec![
        Package {
            id: "pkg1".to_string(),
            repo_id: "repo1".to_string(),
            name: "coreutils".to_string(),
            epoch: "0".to_string(),
            version: "8.32".to_string(),
            release: "33.el9".to_string(),
            arch: "x86_64".to_string(),
            summary: "Core GNU utilities".to_string(),
            sha256: "1234abcd".to_string(),
            size: 5500000,
            location_href: "Packages/c/coreutils-8.32-33.el9.x86_64.rpm".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    let xml = repodata::generate_primary(&original);
    let parsed = repodata::parse_primary(&xml).unwrap();

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "coreutils");
    assert_eq!(parsed[0].version, "8.32");
    assert_eq!(parsed[0].release, "33.el9");
    assert_eq!(parsed[0].sha256, "1234abcd");
    assert_eq!(parsed[0].size, 5500000);
}
