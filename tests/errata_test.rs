//! Tests for updateinfo.xml errata parser.

use stormstar::content::errata;

const UPDATEINFO_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
  <update from="errata@redhat.com" status="final" type="security" version="1">
    <id>RHSA-2023:1234</id>
    <title>Important: bash security update</title>
    <severity>Important</severity>
    <issued date="2023-03-15"/>
    <updated date="2023-03-16"/>
    <description>A security fix for bash that addresses CVE-2023-1234.</description>
    <references>
      <reference href="https://cve.mitre.org" id="CVE-2023-1234" type="cve"/>
      <reference href="https://bugzilla.redhat.com" id="12345" type="bugzilla"/>
    </references>
    <pkglist>
      <collection>
        <package name="bash" version="5.1.8" release="6.el9" arch="x86_64"/>
      </collection>
    </pkglist>
  </update>
  <update from="errata@redhat.com" status="final" type="bugfix" version="1">
    <id>RHBA-2023:5678</id>
    <title>vim-minimal bug fix update</title>
    <severity>None</severity>
    <issued date="2023-04-01"/>
    <updated date="2023-04-01"/>
    <description>Bug fixes for vim-minimal.</description>
    <references/>
    <pkglist>
      <collection>
        <package name="vim-minimal" version="9.0.1" release="1.el9" arch="x86_64"/>
        <package name="vim-common" version="9.0.1" release="1.el9" arch="x86_64"/>
      </collection>
    </pkglist>
  </update>
  <update from="errata@redhat.com" status="final" type="enhancement" version="1">
    <id>RHEA-2023:9999</id>
    <title>New features in curl</title>
    <severity>None</severity>
    <issued date="2023-05-01"/>
    <updated date="2023-05-01"/>
    <description>Enhancement release for curl.</description>
    <references>
      <reference href="https://cve.mitre.org" id="CVE-2023-5555" type="cve"/>
      <reference href="https://cve.mitre.org" id="CVE-2023-6666" type="cve"/>
    </references>
    <pkglist>
      <collection>
        <package name="curl" version="7.76.1" release="26.el9" arch="x86_64"/>
      </collection>
    </pkglist>
  </update>
</updates>"#;

#[test]
fn test_parse_updateinfo() {
    let result = errata::parse_updateinfo(UPDATEINFO_XML).unwrap();

    assert_eq!(result.len(), 3);

    // First: security advisory
    let rhsa = &result[0];
    assert_eq!(rhsa.advisory_id, "RHSA-2023:1234");
    assert_eq!(rhsa.title, "Important: bash security update");
    assert_eq!(rhsa.erratum_type, "Security");
    assert_eq!(rhsa.severity, "Important");
    assert_eq!(rhsa.issued, "2023-03-15");
    assert_eq!(rhsa.updated, "2023-03-16");
    assert_eq!(rhsa.cves, vec!["CVE-2023-1234"]);
    assert_eq!(rhsa.package_names, vec!["bash"]);
    assert!(rhsa.description.contains("CVE-2023-1234"));

    // Second: bugfix
    let rhba = &result[1];
    assert_eq!(rhba.advisory_id, "RHBA-2023:5678");
    assert_eq!(rhba.erratum_type, "Bugfix");
    assert_eq!(rhba.severity, "None");
    assert_eq!(rhba.package_names.len(), 2);
    assert!(rhba.package_names.contains(&"vim-minimal".to_string()));
    assert!(rhba.package_names.contains(&"vim-common".to_string()));
    assert!(rhba.cves.is_empty());

    // Third: enhancement
    let rhea = &result[2];
    assert_eq!(rhea.advisory_id, "RHEA-2023:9999");
    assert_eq!(rhea.erratum_type, "Enhancement");
    assert_eq!(rhea.cves.len(), 2);
    assert!(rhea.cves.contains(&"CVE-2023-5555".to_string()));
    assert!(rhea.cves.contains(&"CVE-2023-6666".to_string()));
}

#[test]
fn test_parse_empty_updateinfo() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
</updates>"#;
    let result = errata::parse_updateinfo(xml).unwrap();
    assert!(result.is_empty());
}
