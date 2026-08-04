#![cfg(windows)]

use std::{ffi::OsString, num::NonZeroUsize, path::PathBuf};

use pastral_agent::{AgentCommand, CliError, parse_arguments};

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn accepts_exact_supported_commands() {
    assert_eq!(
        parse_arguments(args(&["health-check", "--data-root", "C:\\PastralData"])),
        Ok(AgentCommand::HealthCheck {
            data_root: PathBuf::from("C:\\PastralData"),
        })
    );
    assert_eq!(
        parse_arguments(args(&["capture-current", "--data-root", "D:\\Clips"])),
        Ok(AgentCommand::CaptureCurrent {
            data_root: PathBuf::from("D:\\Clips"),
        })
    );
    assert_eq!(
        parse_arguments(args(&["listen", "--data-root", "E:\\History"])),
        Ok(AgentCommand::Listen {
            data_root: PathBuf::from("E:\\History"),
            max_events: None,
        })
    );
    assert_eq!(
        parse_arguments(args(&["run"])),
        Ok(AgentCommand::Run {
            data_root: None,
            max_events: None,
            max_connections: None,
        })
    );
    assert_eq!(
        parse_arguments(args(&[
            "run",
            "--data-root",
            "C:\\Users\\Example\\AppData\\Local\\Pastral",
            "--max-events",
            "2",
            "--max-connections",
            "3",
        ])),
        Ok(AgentCommand::Run {
            data_root: Some(PathBuf::from("C:\\Users\\Example\\AppData\\Local\\Pastral")),
            max_events: Some(NonZeroUsize::new(2).unwrap()),
            max_connections: Some(NonZeroUsize::new(3).unwrap()),
        })
    );
    assert_eq!(
        parse_arguments(args(&[
            "listen",
            "--data-root",
            "E:\\History",
            "--max-events",
            "3",
        ])),
        Ok(AgentCommand::Listen {
            data_root: PathBuf::from("E:\\History"),
            max_events: Some(NonZeroUsize::new(3).unwrap()),
        })
    );
}

#[test]
fn rejects_missing_or_unknown_command_and_root() {
    assert_eq!(parse_arguments(args(&[])), Err(CliError::MissingCommand));
    assert_eq!(
        parse_arguments(args(&["unknown"])),
        Err(CliError::UnknownCommand)
    );
    assert_eq!(
        parse_arguments(args(&["health-check"])),
        Err(CliError::MissingDataRoot)
    );
    assert_eq!(
        parse_arguments(args(&["health-check", "--data-root"])),
        Err(CliError::MissingFlagValue("--data-root"))
    );
}

#[test]
fn rejects_zero_duplicate_unknown_and_positional_arguments() {
    assert_eq!(
        parse_arguments(args(&[
            "listen",
            "--data-root",
            "C:\\Data",
            "--max-events",
            "0",
        ])),
        Err(CliError::InvalidMaxEvents)
    );
    assert_eq!(
        parse_arguments(args(&[
            "listen",
            "--data-root",
            "C:\\Data",
            "--data-root",
            "D:\\Other",
        ])),
        Err(CliError::DuplicateFlag("--data-root"))
    );
    assert_eq!(
        parse_arguments(args(&[
            "listen",
            "--data-root",
            "C:\\Data",
            "--max-events",
            "2",
            "--max-events",
            "3",
        ])),
        Err(CliError::DuplicateFlag("--max-events"))
    );
    assert_eq!(
        parse_arguments(args(&["health-check", "--data-root", "C:\\Data", "--bad"])),
        Err(CliError::UnknownFlag)
    );
    assert_eq!(
        parse_arguments(args(&[
            "capture-current",
            "--data-root",
            "C:\\Data",
            "extra"
        ])),
        Err(CliError::UnexpectedArgument)
    );
    assert_eq!(
        parse_arguments(args(&[
            "health-check",
            "--data-root",
            "C:\\Data",
            "--max-events",
            "2",
        ])),
        Err(CliError::FlagNotAllowed("--max-events"))
    );
    assert_eq!(
        parse_arguments(args(&["run", "--max-connections", "0"])),
        Err(CliError::InvalidMaxConnections)
    );
    assert_eq!(
        parse_arguments(args(&["run", "--max-connections", "17"])),
        Err(CliError::InvalidMaxConnections)
    );
    assert_eq!(
        parse_arguments(args(&[
            "run",
            "--max-connections",
            "2",
            "--max-connections",
            "3",
        ])),
        Err(CliError::DuplicateFlag("--max-connections"))
    );
    assert_eq!(
        parse_arguments(args(&[
            "listen",
            "--data-root",
            "C:\\Data",
            "--max-connections",
            "2",
        ])),
        Err(CliError::FlagNotAllowed("--max-connections"))
    );
}
