#![cfg(windows)]

use std::process::Command;

#[test]
fn parent_runs_distinct_authenticated_agent_health_server() {
    let executable = env!("CARGO_BIN_EXE_pastral-agent-ipc-probe");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("agent-ipc-admission=ok\n"));
    assert!(stdout.contains("cross-process=true\n"));
    assert!(stdout.contains("health=ok\n"));
    assert!(stdout.contains("admission-ceilings=debug-not-enforced\n"));
    let client = metric(&stdout, "client-pid");
    let server = metric(&stdout, "server-pid");
    assert_ne!(client, server);
    assert!(metric(&stdout, "session-id") <= u128::from(u32::MAX));
    for key in [
        "default-agent-binary-bytes",
        "admission-binary-bytes",
        "baseline-working-set-bytes",
        "baseline-private-bytes",
        "server-working-set-bytes",
        "server-private-bytes",
        "connect-us",
        "handshake-us",
        "health-us",
        "total-us",
    ] {
        assert!(metric(&stdout, key) > 0);
    }
    for key in [
        "binary-delta-bytes",
        "working-set-delta-bytes",
        "private-delta-bytes",
    ] {
        assert!(signed_metric(&stdout, key).is_some());
    }
    assert_content_free(&stdout);
}

#[test]
fn parent_runs_distinct_authenticated_read_server() {
    let executable = env!("CARGO_BIN_EXE_pastral-agent-ipc-probe");
    let output = Command::new(executable)
        .arg("--read-probe")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("agent-ipc-read=ok\n"));
    assert!(stdout.contains("cross-process=true\n"));
    assert!(stdout.contains("health=ok\n"));
    assert!(stdout.contains("history=ok\n"));
    assert!(stdout.contains("search=ok\n"));
    let client = metric(&stdout, "client-pid");
    let server = metric(&stdout, "server-pid");
    assert_ne!(client, server);
    assert!(metric(&stdout, "session-id") <= u128::from(u32::MAX));
    assert_content_free(&stdout);
}

#[test]
fn invalid_arguments_fail_before_child_or_transport_creation() {
    let executable = env!("CARGO_BIN_EXE_pastral-agent-ipc-probe");
    let output = Command::new(executable).arg("--unknown").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "agent-ipc-admission=invalid arguments\n"
    );
}

fn assert_content_free(output: &str) {
    let lower = output.to_ascii_lowercase();
    for forbidden in [
        "\\\\.\\pipe\\",
        "secret=",
        "nonce=",
        "proof=",
        "root=",
        "sid=",
        "clipboard",
        "preview=",
        "query=",
    ] {
        assert!(!lower.contains(&forbidden.to_ascii_lowercase()));
    }
}

fn metric(output: &str, key: &str) -> u128 {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing metric {key}"))
        .parse::<u128>()
        .unwrap()
}

fn signed_metric(output: &str, key: &str) -> Option<i128> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(|value| value.parse::<i128>().unwrap())
}
