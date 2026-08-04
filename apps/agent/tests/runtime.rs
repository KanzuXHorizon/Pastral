#![cfg(windows)]

use std::{fs, path::PathBuf};

use pastral_agent::{AgentCommand, run_command};
use pastral_domain::ClipEventId;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-runtime-test-{}",
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
fn health_check_reports_content_free_integrity_markers() {
    let root = TestRoot::new();
    let mut output = Vec::new();

    run_command(
        AgentCommand::HealthCheck {
            data_root: root.path().to_path_buf(),
        },
        &mut output,
    )
    .unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("agent-health=ok"));
    assert!(output.contains("privacy-policy=ok"));
    assert!(output.contains("storage-schema=1"));
    assert!(output.contains("sqlite-integrity=ok"));
    assert!(output.contains("fts-integrity=ok"));
    assert!(output.contains("metadata-integrity=ok"));
    assert!(output.contains("search-mapping-integrity=ok"));
    assert!(!output.to_ascii_lowercase().contains("clipboard-text"));
    assert!(!output.to_ascii_lowercase().contains("content-hash"));
    assert!(root.path().join("agent-identity.txt").is_file());
    assert!(root.path().join("privacy-policy.txt").is_file());
    assert!(
        root.path()
            .join("storage")
            .join("metadata.sqlite3")
            .is_file()
    );
}
