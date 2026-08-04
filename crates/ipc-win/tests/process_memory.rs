#![cfg(windows)]

use std::process::Command;

use pastral_ipc_win::process_memory_snapshot;

#[test]
fn current_process_memory_is_nonzero_and_repeatable() {
    let process_id = std::process::id();
    for _ in 0..32 {
        let snapshot = process_memory_snapshot(process_id).unwrap();
        assert!(snapshot.working_set_bytes() > 0);
        assert!(snapshot.private_usage_bytes() > 0);
    }
}

#[test]
fn zero_invalid_and_exited_processes_fail_closed() {
    assert!(process_memory_snapshot(0).is_err());
    assert!(process_memory_snapshot(u32::MAX).is_err());

    let mut child = Command::new("cmd.exe")
        .args(["/d", "/c", "exit", "0"])
        .spawn()
        .unwrap();
    let process_id = child.id();
    assert!(child.wait().unwrap().success());
    drop(child);
    assert!(process_memory_snapshot(process_id).is_err());
}
