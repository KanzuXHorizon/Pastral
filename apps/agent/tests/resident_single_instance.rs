#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn(executable: &str, root: &Path) -> Self {
        let child = Command::new(executable)
            .arg("run")
            .arg("--data-root")
            .arg(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("resident process must start");
        Self(Some(child))
    }

    fn child_mut(&mut self) -> &mut Child {
        self.0.as_mut().expect("child must exist")
    }

    fn stop(mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-resident-instance-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wait_for_storage(root: &Path, child: &mut Child) {
    let metadata = root.join("storage").join("metadata.sqlite3");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        assert!(
            child.try_wait().unwrap().is_none(),
            "resident exited during startup"
        );
        if metadata.is_file() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("resident did not initialize storage within timeout");
}

#[test]
fn second_resident_for_same_root_exits_cleanly_without_becoming_an_owner() {
    let executable = env!("CARGO_BIN_EXE_pastral-agent");
    let root = TemporaryRoot::new();

    let mut first = ChildGuard::spawn(executable, root.path());
    wait_for_storage(root.path(), first.child_mut());

    let second = Command::new(executable)
        .arg("run")
        .arg("--data-root")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(second.status.success());
    assert!(second.stderr.is_empty());
    assert_eq!(
        String::from_utf8(second.stdout).unwrap(),
        "resident-instance=already-running\n"
    );
    assert!(first.child_mut().try_wait().unwrap().is_none());

    first.stop();

    let mut replacement = ChildGuard::spawn(executable, root.path());
    wait_for_storage(root.path(), replacement.child_mut());
    assert!(replacement.child_mut().try_wait().unwrap().is_none());
    replacement.stop();
}
