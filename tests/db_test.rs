//! Tests for database CRUD operations.

use std::sync::Arc;
use stormstar::db;
use stormstar::db::models::*;

fn temp_db() -> Arc<native_db::Database<'static>> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    // Keep the tempdir alive by leaking it (test-only)
    let path_str = path.to_str().unwrap().to_string();
    std::mem::forget(dir);
    Arc::new(db::open_db(&path_str).unwrap())
}

#[test]
fn test_organization_crud() {
    let db = temp_db();

    // Create
    let org = Organization::new("TestOrg", "test_org");
    let rw = db.rw_transaction().unwrap();
    rw.insert(org.clone()).unwrap();
    rw.commit().unwrap();

    // Read
    let r = db.r_transaction().unwrap();
    let loaded: Organization = r.get().primary(org.id.clone()).unwrap().unwrap();
    assert_eq!(loaded.name, "TestOrg");
    assert_eq!(loaded.label, "test_org");

    // Update
    let rw = db.rw_transaction().unwrap();
    let old: Organization = rw.get().primary(org.id.clone()).unwrap().unwrap();
    let mut updated = old.clone();
    updated.description = "Updated description".to_string();
    rw.update(old, updated.clone()).unwrap();
    rw.commit().unwrap();

    let r = db.r_transaction().unwrap();
    let loaded: Organization = r.get().primary(org.id.clone()).unwrap().unwrap();
    assert_eq!(loaded.description, "Updated description");

    // Delete
    let rw = db.rw_transaction().unwrap();
    let item: Organization = rw.get().primary(org.id.clone()).unwrap().unwrap();
    rw.remove(item).unwrap();
    rw.commit().unwrap();

    let r = db.r_transaction().unwrap();
    let gone: Option<Organization> = r.get().primary(org.id).unwrap();
    assert!(gone.is_none());
}

#[test]
fn test_repository_crud() {
    let db = temp_db();

    let repo = Repository::new("prod-1", "CentOS Base", "https://mirror.centos.org/centos/9-stream/BaseOS/x86_64/os/");
    let rw = db.rw_transaction().unwrap();
    rw.insert(repo.clone()).unwrap();
    rw.commit().unwrap();

    let r = db.r_transaction().unwrap();
    let loaded: Repository = r.get().primary(repo.id.clone()).unwrap().unwrap();
    assert_eq!(loaded.name, "CentOS Base");
    assert_eq!(loaded.sync_state, RepoSyncState::NotSynced);
    assert_eq!(loaded.package_count, 0);
}

#[test]
fn test_package_nevra() {
    let pkg = Package {
        id: "1".to_string(),
        repo_id: "r1".to_string(),
        name: "bash".to_string(),
        epoch: "0".to_string(),
        version: "5.1.8".to_string(),
        release: "6.el9".to_string(),
        arch: "x86_64".to_string(),
        summary: String::new(),
        sha256: String::new(),
        size: 0,
        location_href: String::new(),
        downloaded: false,
        local_path: String::new(),
        download_size: 0,
        created_at: String::new(),
    };
    assert_eq!(pkg.nevra(), "bash-5.1.8-6.el9.x86_64");
    assert_eq!(pkg.filename(), "bash-5.1.8-6.el9.x86_64.rpm");

    // With non-zero epoch
    let pkg2 = Package {
        epoch: "2".to_string(),
        name: "vim".to_string(),
        ..pkg
    };
    assert_eq!(pkg2.nevra(), "2:vim-5.1.8-6.el9.x86_64");
}

#[test]
fn test_activation_key_generation() {
    let key = ActivationKey::new("org-1", "dev-key", "env-1", "cv-1");
    assert!(key.key.starts_with("dev-key-"));
    assert_eq!(key.usage_count, 0);
    assert!(key.max_hosts.is_none());
}

#[test]
fn test_content_view_create() {
    let cv = ContentView::new("org-1", "Base OS View");
    assert_eq!(cv.label, "base_os_view");
    assert_eq!(cv.latest_version, 0);
    assert!(cv.repo_ids.is_empty());
}

#[test]
fn test_lifecycle_environment_chain() {
    let db = temp_db();

    let env1 = LifecycleEnvironment::new("org-1", "Library", 0, None);
    let env2 = LifecycleEnvironment::new("org-1", "Development", 1, Some(&env1.id));
    let env3 = LifecycleEnvironment::new("org-1", "Production", 2, Some(&env2.id));

    let rw = db.rw_transaction().unwrap();
    rw.insert(env1.clone()).unwrap();
    rw.insert(env2.clone()).unwrap();
    rw.insert(env3.clone()).unwrap();
    rw.commit().unwrap();

    let r = db.r_transaction().unwrap();
    let loaded: LifecycleEnvironment = r.get().primary(env2.id.clone()).unwrap().unwrap();
    assert_eq!(loaded.name, "Development");
    assert_eq!(loaded.prior_id, Some(env1.id));
    assert_eq!(loaded.position, 1);
}

#[test]
fn test_host_collection() {
    let coll = HostCollection::new("org-1", "Web Servers");
    assert_eq!(coll.name, "Web Servers");
    assert!(coll.host_ids.is_empty());
}

#[test]
fn test_scan_all() {
    let db = temp_db();

    let rw = db.rw_transaction().unwrap();
    rw.insert(Organization::new("Org1", "org1")).unwrap();
    rw.insert(Organization::new("Org2", "org2")).unwrap();
    rw.insert(Organization::new("Org3", "org3")).unwrap();
    rw.commit().unwrap();

    let r = db.r_transaction().unwrap();
    let all: Vec<Organization> = r.scan().primary()
        .unwrap()
        .all()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(all.len(), 3);
}
