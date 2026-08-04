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
    let client = metric(&stdout, "client-pid");
    let server = metric(&stdout, "server-pid");
    assert_ne!(client, server);
    assert!(metric(&stdout, "session-id") <= u128::from(u32::MAX));
    for key in ["connect-us", "handshake-us", "health-us", "total-us"] {
        assert!(metric(&stdout, key) > 0);
    }
    let lower = stdout.to_ascii_lowercase();
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

fn metric(output: &str, key: &str) -> u128 {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing metric {key}"))
        .parse::<u128>()
        .unwrap()
}
