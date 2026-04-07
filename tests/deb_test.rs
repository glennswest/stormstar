//! Tests for APT (deb) metadata parsing and generation.

use stormstar::content::deb;
use stormstar::db::models::Package;

const PACKAGES_TEXT: &str = r#"Package: curl
Version: 7.88.1-10+deb12u8
Architecture: amd64
Filename: pool/main/c/curl/curl_7.88.1-10+deb12u8_amd64.deb
Size: 321468
SHA256: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
Description: command line tool for transferring data with URL syntax
Section: web
Source: curl

Package: libssl3
Version: 2:3.0.13-1~deb12u2
Architecture: amd64
Filename: pool/main/o/openssl/libssl3_3.0.13-1~deb12u2_amd64.deb
Size: 2019340
SHA256: ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00
Description: Secure Sockets Layer toolkit - shared libraries
Section: libs
Source: openssl

Package: bash
Version: 5.2.15-2+b7
Architecture: amd64
Filename: pool/main/b/bash/bash_5.2.15-2+b7_amd64.deb
Size: 1416024
SHA256: deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef
Description: GNU Bourne Again SHell
Section: shells
Source: bash
"#;

#[test]
fn test_parse_packages() {
    let packages = deb::parse_packages(PACKAGES_TEXT);

    assert_eq!(packages.len(), 3);

    let curl = &packages[0];
    assert_eq!(curl.package, "curl");
    assert_eq!(curl.version, "7.88.1-10+deb12u8");
    assert_eq!(curl.architecture, "amd64");
    assert_eq!(curl.size, 321468);
    assert_eq!(curl.sha256, "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2");
    assert_eq!(curl.filename, "pool/main/c/curl/curl_7.88.1-10+deb12u8_amd64.deb");
    assert_eq!(curl.section, "web");
    assert_eq!(curl.source, "curl");

    let libssl = &packages[1];
    assert_eq!(libssl.package, "libssl3");
    assert_eq!(libssl.version, "2:3.0.13-1~deb12u2");
    assert_eq!(libssl.size, 2019340);

    let bash = &packages[2];
    assert_eq!(bash.package, "bash");
    assert_eq!(bash.section, "shells");
}

#[test]
fn test_parse_deb_version() {
    // epoch:upstream-revision
    let (epoch, upstream, revision) = deb::parse_deb_version("2:1.18.0-6ubuntu14");
    assert_eq!(epoch, "2");
    assert_eq!(upstream, "1.18.0");
    assert_eq!(revision, "6ubuntu14");

    // no epoch
    let (epoch, upstream, revision) = deb::parse_deb_version("1.2.3-1");
    assert_eq!(epoch, "0");
    assert_eq!(upstream, "1.2.3");
    assert_eq!(revision, "1");

    // no revision
    let (epoch, upstream, revision) = deb::parse_deb_version("1.0");
    assert_eq!(epoch, "0");
    assert_eq!(upstream, "1.0");
    assert_eq!(revision, "");

    // complex revision with tilde
    let (epoch, upstream, revision) = deb::parse_deb_version("3.0.13-1~deb12u2");
    assert_eq!(epoch, "0");
    assert_eq!(upstream, "3.0.13");
    assert_eq!(revision, "1~deb12u2");
}

const RELEASE_TEXT: &str = r#"Origin: Debian
Label: Debian
Suite: stable
Codename: bookworm
Architectures: amd64 arm64 i386
Components: main contrib non-free non-free-firmware
Date: Sat, 01 Mar 2025 00:00:00 UTC
SHA256:
 abc123def456 12345 main/binary-amd64/Packages
 789012fed345 67890 main/binary-amd64/Packages.gz
 aaabbb111222 34567 contrib/binary-amd64/Packages
"#;

#[test]
fn test_parse_release() {
    let release = deb::parse_release(RELEASE_TEXT);

    assert_eq!(release.origin, "Debian");
    assert_eq!(release.label, "Debian");
    assert_eq!(release.suite, "stable");
    assert_eq!(release.codename, "bookworm");
    assert_eq!(release.architectures, vec!["amd64", "arm64", "i386"]);
    assert_eq!(release.components, vec!["main", "contrib", "non-free", "non-free-firmware"]);
    assert_eq!(release.sha256_entries.len(), 3);
    assert_eq!(release.sha256_entries[0].hash, "abc123def456");
    assert_eq!(release.sha256_entries[0].size, 12345);
    assert_eq!(release.sha256_entries[0].filename, "main/binary-amd64/Packages");
}

#[test]
fn test_generate_packages() {
    let packages = vec![
        Package {
            id: "pkg1".to_string(),
            repo_id: "repo1".to_string(),
            name: "curl".to_string(),
            epoch: "0".to_string(),
            version: "7.88.1-10+deb12u8".to_string(),
            release: "10+deb12u8".to_string(),
            arch: "amd64".to_string(),
            summary: "command line tool for transferring data".to_string(),
            sha256: "aabbccdd".to_string(),
            size: 321468,
            location_href: "pool/main/c/curl/curl_7.88.1_amd64.deb".to_string(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    let text = deb::generate_packages(&packages, "main");
    assert!(text.contains("Package: curl"));
    assert!(text.contains("Version: 7.88.1-10+deb12u8"));
    assert!(text.contains("Architecture: amd64"));
    assert!(text.contains("Size: 321468"));
    assert!(text.contains("SHA256: aabbccdd"));
    assert!(text.contains("Description: command line tool for transferring data"));
}

#[test]
fn test_generate_release() {
    let archs = vec!["amd64".to_string(), "arm64".to_string()];
    let components = vec!["main".to_string(), "universe".to_string()];
    let entries = vec![
        ("main/binary-amd64/Packages".to_string(), b"package data here".to_vec()),
    ];

    let text = deb::generate_release("jammy", &archs, &components, &entries);
    assert!(text.contains("Origin: StormStar"));
    assert!(text.contains("Codename: jammy"));
    assert!(text.contains("Architectures: amd64 arm64"));
    assert!(text.contains("Components: main universe"));
    assert!(text.contains("SHA256:"));
    assert!(text.contains("main/binary-amd64/Packages"));
}

#[test]
fn test_roundtrip_packages() {
    let packages = vec![
        Package {
            id: "pkg1".to_string(),
            repo_id: "repo1".to_string(),
            name: "nginx".to_string(),
            epoch: "0".to_string(),
            version: "1.24.0-2".to_string(),
            release: "2".to_string(),
            arch: "amd64".to_string(),
            summary: "small, powerful, scalable web/proxy server".to_string(),
            sha256: "1234abcd5678ef90".to_string(),
            size: 620000,
            location_href: String::new(),
            downloaded: false,
            local_path: String::new(),
            download_size: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    // Generate → parse → verify
    let text = deb::generate_packages(&packages, "main");
    let parsed = deb::parse_packages(&text);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].package, "nginx");
    assert_eq!(parsed[0].version, "1.24.0-2");
    assert_eq!(parsed[0].architecture, "amd64");
    assert_eq!(parsed[0].sha256, "1234abcd5678ef90");
    assert_eq!(parsed[0].size, 620000);
    assert!(parsed[0].description.contains("small, powerful"));
}

#[test]
fn test_decompress_packages_gz() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let original = "Package: test\nVersion: 1.0\n\n";
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let decompressed = deb::decompress_packages_gz(&compressed).unwrap();
    assert_eq!(decompressed, original);
}
