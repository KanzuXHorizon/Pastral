#![cfg(windows)]

use std::{fs, io::Cursor, path::PathBuf};

use pastral_agent_ipc_probe::run_baseline_child;
use pastral_domain::ClipEventId;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-ipc-baseline-{}",
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
fn baseline_child_loads_real_health_and_exits_on_empty_stdin() {
    let root = TestRoot::new();
    let mut output = Vec::new();

    run_baseline_child(root.path(), Cursor::new(Vec::<u8>::new()), &mut output).unwrap();

    assert_eq!(output, b"agent-baseline-ready=ok\n");
    assert!(root.path().join("agent-identity.txt").is_file());
    assert!(root.path().join("privacy-policy.txt").is_file());
    assert!(
        root.path()
            .join("storage")
            .join("metadata.sqlite3")
            .is_file()
    );
    assert!(!root.path().join("ipc-transport-identity.txt").exists());
    assert!(!root.path().join("ipc-installation-secret.dpapi").exists());
}

#[test]
fn baseline_child_rejects_stdin_as_a_command_channel() {
    let root = TestRoot::new();
    let mut output = Vec::new();

    assert!(run_baseline_child(root.path(), Cursor::new(b"command"), &mut output).is_err());
    assert_eq!(output, b"agent-baseline-ready=ok\n");
}
