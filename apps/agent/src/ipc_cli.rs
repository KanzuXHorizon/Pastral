use core::fmt;
use std::{ffi::OsString, num::NonZeroUsize, path::PathBuf};

const MAX_CONNECTIONS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIpcCommand {
    ServeHealth {
        data_root: PathBuf,
        max_connections: NonZeroUsize,
    },
    ServeRead {
        data_root: PathBuf,
        max_connections: NonZeroUsize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentIpcCliError {
    MissingCommand,
    UnknownCommand,
    MissingDataRoot,
    MissingFlagValue(&'static str),
    DuplicateFlag(&'static str),
    UnknownFlag,
    InvalidMaxConnections,
    UnexpectedArgument,
}

impl fmt::Display for AgentIpcCliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("missing command"),
            Self::UnknownCommand => formatter.write_str("unknown command"),
            Self::MissingDataRoot => formatter.write_str("missing --data-root"),
            Self::MissingFlagValue(flag) => write!(formatter, "missing value for {flag}"),
            Self::DuplicateFlag(flag) => write!(formatter, "duplicate flag {flag}"),
            Self::UnknownFlag => formatter.write_str("unknown flag"),
            Self::InvalidMaxConnections => {
                write!(
                    formatter,
                    "--max-connections must be in 1..={MAX_CONNECTIONS}"
                )
            }
            Self::UnexpectedArgument => formatter.write_str("unexpected positional argument"),
        }
    }
}

impl std::error::Error for AgentIpcCliError {}

pub fn parse_ipc_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<AgentIpcCommand, AgentIpcCliError> {
    let mut arguments = arguments.into_iter();
    let command = arguments.next().ok_or(AgentIpcCliError::MissingCommand)?;
    let command = match command.to_str() {
        Some("serve-health") => IpcCommandKind::Health,
        Some("serve-read") => IpcCommandKind::Read,
        _ => return Err(AgentIpcCliError::UnknownCommand),
    };

    let mut data_root = None;
    let mut max_connections = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--data-root") => {
                if data_root.is_some() {
                    return Err(AgentIpcCliError::DuplicateFlag("--data-root"));
                }
                let value = arguments
                    .next()
                    .ok_or(AgentIpcCliError::MissingFlagValue("--data-root"))?;
                if value.is_empty() {
                    return Err(AgentIpcCliError::MissingFlagValue("--data-root"));
                }
                data_root = Some(PathBuf::from(value));
            }
            Some("--max-connections") => {
                if max_connections.is_some() {
                    return Err(AgentIpcCliError::DuplicateFlag("--max-connections"));
                }
                let value = arguments
                    .next()
                    .ok_or(AgentIpcCliError::MissingFlagValue("--max-connections"))?;
                let parsed = value
                    .to_str()
                    .ok_or(AgentIpcCliError::InvalidMaxConnections)?
                    .parse::<usize>()
                    .map_err(|_| AgentIpcCliError::InvalidMaxConnections)?;
                let parsed = NonZeroUsize::new(parsed)
                    .filter(|value| value.get() <= MAX_CONNECTIONS)
                    .ok_or(AgentIpcCliError::InvalidMaxConnections)?;
                max_connections = Some(parsed);
            }
            Some(value) if value.starts_with('-') => return Err(AgentIpcCliError::UnknownFlag),
            _ => return Err(AgentIpcCliError::UnexpectedArgument),
        }
    }

    let data_root = data_root.ok_or(AgentIpcCliError::MissingDataRoot)?;
    let max_connections = max_connections.unwrap_or(NonZeroUsize::MIN);
    Ok(match command {
        IpcCommandKind::Health => AgentIpcCommand::ServeHealth {
            data_root,
            max_connections,
        },
        IpcCommandKind::Read => AgentIpcCommand::ServeRead {
            data_root,
            max_connections,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpcCommandKind {
    Health,
    Read,
}

#[must_use]
pub const fn ipc_usage() -> &'static str {
    "Usage:\n  pastral-agent-ipc serve-health --data-root <path> [--max-connections <1..=16>]\n  pastral-agent-ipc serve-read --data-root <path> [--max-connections <1..=16>]"
}
