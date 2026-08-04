use std::process::Command;

#[test]
fn release_shape_runs_two_processes_and_emits_only_content_free_metrics() {
    let executable = env!("CARGO_BIN_EXE_pastral-ipc-transport-probe");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ipc-transport-probe=ok\n"));
    assert!(stdout.contains("cross-process=true\n"));
    let client = metric(&stdout, "client-pid");
    let server = metric(&stdout, "server-pid");
    assert_ne!(client, server);
    assert!(metric(&stdout, "session-id") <= u128::from(u32::MAX));
    for key in ["connect-us", "handshake-us", "health-us", "total-us"] {
        assert!(metric(&stdout, key) > 0);
    }
    for forbidden in [
        "\\\\.\\pipe\\",
        "ipc-installation-secret",
        "ipc-transport-identity",
        "secret=",
        "nonce=",
        "proof=",
        "root=",
        "sid=",
    ] {
        assert!(
            !stdout
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase())
        );
    }
}

#[test]
fn invalid_arguments_fail_closed_without_starting_transport() {
    let executable = env!("CARGO_BIN_EXE_pastral-ipc-transport-probe");
    let output = Command::new(executable).arg("--unknown").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "ipc-transport-probe=invalid arguments\n"
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
