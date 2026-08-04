#![cfg(windows)]

use std::ffi::OsString;

use pastral_agent_ipc_probe::{AdmissionError, AdmissionMode, parse_arguments};

#[test]
fn accepts_only_parent_read_parent_and_exact_child_shapes() {
    assert_eq!(
        parse_arguments(Vec::<OsString>::new()).unwrap(),
        AdmissionMode::Parent
    );
    assert_eq!(
        parse_arguments(["--read-probe".into()]).unwrap(),
        AdmissionMode::ReadParent
    );
    assert_eq!(
        parse_arguments([
            "--baseline-child".into(),
            "--data-root".into(),
            "C:\\temp\\baseline".into(),
        ])
        .unwrap(),
        AdmissionMode::BaselineChild {
            data_root: "C:\\temp\\baseline".into(),
        }
    );
    assert_eq!(
        parse_arguments([
            "--server-child".into(),
            "--data-root".into(),
            "C:\\temp\\server".into(),
        ])
        .unwrap(),
        AdmissionMode::ServerChild {
            data_root: "C:\\temp\\server".into(),
        }
    );
    assert_eq!(
        parse_arguments([
            "--read-server-child".into(),
            "--data-root".into(),
            "C:\\temp\\read-server".into(),
        ])
        .unwrap(),
        AdmissionMode::ReadServerChild {
            data_root: "C:\\temp\\read-server".into(),
        }
    );
}

#[test]
fn missing_unknown_empty_duplicate_and_positional_arguments_fail_closed() {
    let invalid = [
        vec!["--baseline-child".into()],
        vec!["--server-child".into(), "--data-root".into()],
        vec![
            "--server-child".into(),
            "--data-root".into(),
            OsString::new(),
        ],
        vec!["--unknown".into()],
        vec!["value".into()],
        vec![
            "--baseline-child".into(),
            "--data-root".into(),
            "a".into(),
            "--data-root".into(),
            "b".into(),
        ],
        vec![
            "--server-child".into(),
            "--data-root".into(),
            "a".into(),
            "extra".into(),
        ],
    ];

    for arguments in invalid {
        assert_eq!(
            parse_arguments(arguments),
            Err(AdmissionError::InvalidArguments)
        );
    }
}
