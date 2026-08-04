#![cfg(windows)]

use std::{fs, path::PathBuf};

use pastral_agent::load_health_snapshot;
use pastral_domain::ClipEventId;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-health-{label}-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn real_disposable_root_returns_content_free_healthy_snapshot() {
    let root = TestRoot::new("healthy");
    let snapshot = load_health_snapshot(root.path()).unwrap();

    assert_eq!(snapshot.storage_schema_version(), 1);
    assert!(!snapshot.capture_enabled());
    assert!(snapshot.privacy_policy_ok());
    assert!(snapshot.storage_integrity_ok());
    assert!(root.path().join("agent-identity.txt").is_file());
    assert!(root.path().join("privacy-policy.txt").is_file());
    assert!(
        root.path()
            .join("storage")
            .join("metadata.sqlite3")
            .is_file()
    );
}

#[test]
fn health_snapshot_source_has_no_content_bearing_fields_or_debug_derive() {
    let source = include_str!("../src/health.rs");
    let struct_start = source.find("pub struct AgentHealthSnapshot").unwrap();
    let struct_end = source[struct_start..].find("\n}\n").unwrap() + struct_start;
    let snapshot_definition = &source[struct_start..=struct_end];
    assert!(!snapshot_definition.contains("derive(Debug"));
    for forbidden in [
        "data_root:",
        "preview:",
        "query:",
        "source:",
        "digest:",
        "clip_count:",
    ] {
        assert!(!snapshot_definition.contains(forbidden));
    }
}
