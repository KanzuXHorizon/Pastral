#![cfg(all(windows, feature = "ipc-health"))]

use std::{ffi::OsString, num::NonZeroUsize, path::PathBuf};

use pastral_agent::{AgentIpcCliError, AgentIpcCommand, parse_ipc_arguments};

#[test]
fn accepts_exact_serve_health_and_read_shapes() {
    assert_eq!(
        parse_ipc_arguments([
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from(r"C:\temp\pastral"),
        ]),
        Ok(AgentIpcCommand::ServeHealth {
            data_root: PathBuf::from(r"C:\temp\pastral"),
            max_connections: NonZeroUsize::MIN,
        })
    );

    assert_eq!(
        parse_ipc_arguments([
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from(r"C:\temp\pastral"),
            OsString::from("--max-connections"),
            OsString::from("16"),
        ]),
        Ok(AgentIpcCommand::ServeHealth {
            data_root: PathBuf::from(r"C:\temp\pastral"),
            max_connections: NonZeroUsize::new(16).unwrap(),
        })
    );

    assert_eq!(
        parse_ipc_arguments([
            OsString::from("serve-read"),
            OsString::from("--data-root"),
            OsString::from(r"C:\temp\pastral"),
            OsString::from("--max-connections"),
            OsString::from("3"),
        ]),
        Ok(AgentIpcCommand::ServeRead {
            data_root: PathBuf::from(r"C:\temp\pastral"),
            max_connections: NonZeroUsize::new(3).unwrap(),
        })
    );
}

#[test]
fn rejects_missing_unknown_duplicate_positional_and_out_of_range_arguments() {
    let invalid = [
        vec![],
        vec![OsString::from("unknown")],
        vec![OsString::from("serve-health")],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::new(),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from("a"),
            OsString::from("--data-root"),
            OsString::from("b"),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from("a"),
            OsString::from("--max-connections"),
            OsString::from("0"),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from("a"),
            OsString::from("--max-connections"),
            OsString::from("17"),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from("a"),
            OsString::from("--unknown"),
        ],
        vec![
            OsString::from("serve-health"),
            OsString::from("--data-root"),
            OsString::from("a"),
            OsString::from("positional"),
        ],
    ];

    for arguments in invalid {
        assert!(matches!(
            parse_ipc_arguments(arguments),
            Err(AgentIpcCliError::MissingCommand
                | AgentIpcCliError::UnknownCommand
                | AgentIpcCliError::MissingDataRoot
                | AgentIpcCliError::MissingFlagValue(_)
                | AgentIpcCliError::DuplicateFlag(_)
                | AgentIpcCliError::UnknownFlag
                | AgentIpcCliError::InvalidMaxConnections
                | AgentIpcCliError::UnexpectedArgument)
        ));
    }
}
