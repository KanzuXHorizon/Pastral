use core::fmt;
use std::{ffi::OsString, num::NonZeroUsize, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentCommand {
    HealthCheck {
        data_root: PathBuf,
    },
    CaptureCurrent {
        data_root: PathBuf,
    },
    Listen {
        data_root: PathBuf,
        max_events: Option<NonZeroUsize>,
    },
}

impl AgentCommand {
    #[must_use]
    pub fn data_root(&self) -> &PathBuf {
        match self {
            Self::HealthCheck { data_root }
            | Self::CaptureCurrent { data_root }
            | Self::Listen { data_root, .. } => data_root,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliError {
    MissingCommand,
    UnknownCommand,
    MissingDataRoot,
    MissingFlagValue(&'static str),
    DuplicateFlag(&'static str),
    UnknownFlag,
    FlagNotAllowed(&'static str),
    InvalidMaxEvents,
    UnexpectedArgument,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing command"),
            Self::UnknownCommand => write!(f, "unknown command"),
            Self::MissingDataRoot => write!(f, "missing --data-root"),
            Self::MissingFlagValue(flag) => write!(f, "missing value for {flag}"),
            Self::DuplicateFlag(flag) => write!(f, "duplicate flag {flag}"),
            Self::UnknownFlag => write!(f, "unknown flag"),
            Self::FlagNotAllowed(flag) => write!(f, "flag not allowed for command: {flag}"),
            Self::InvalidMaxEvents => write!(f, "--max-events must be a positive integer"),
            Self::UnexpectedArgument => write!(f, "unexpected positional argument"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<AgentCommand, CliError> {
    let mut arguments = arguments.into_iter();
    let command = arguments.next().ok_or(CliError::MissingCommand)?;
    let command = command.to_str().ok_or(CliError::UnknownCommand)?;
    let allows_max_events = match command {
        "health-check" | "capture-current" => false,
        "listen" => true,
        _ => return Err(CliError::UnknownCommand),
    };

    let mut data_root = None;
    let mut max_events = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--data-root") => {
                if data_root.is_some() {
                    return Err(CliError::DuplicateFlag("--data-root"));
                }
                let value = arguments
                    .next()
                    .ok_or(CliError::MissingFlagValue("--data-root"))?;
                if value.is_empty() {
                    return Err(CliError::MissingFlagValue("--data-root"));
                }
                data_root = Some(PathBuf::from(value));
            }
            Some("--max-events") => {
                if !allows_max_events {
                    return Err(CliError::FlagNotAllowed("--max-events"));
                }
                if max_events.is_some() {
                    return Err(CliError::DuplicateFlag("--max-events"));
                }
                let value = arguments
                    .next()
                    .ok_or(CliError::MissingFlagValue("--max-events"))?;
                let value = value
                    .to_str()
                    .ok_or(CliError::InvalidMaxEvents)?
                    .parse::<usize>()
                    .map_err(|_| CliError::InvalidMaxEvents)?;
                max_events = Some(NonZeroUsize::new(value).ok_or(CliError::InvalidMaxEvents)?);
            }
            Some(value) if value.starts_with('-') => return Err(CliError::UnknownFlag),
            _ => return Err(CliError::UnexpectedArgument),
        }
    }

    let data_root = data_root.ok_or(CliError::MissingDataRoot)?;
    match command {
        "health-check" => Ok(AgentCommand::HealthCheck { data_root }),
        "capture-current" => Ok(AgentCommand::CaptureCurrent { data_root }),
        "listen" => Ok(AgentCommand::Listen {
            data_root,
            max_events,
        }),
        _ => unreachable!("command was validated before parsing flags"),
    }
}

#[must_use]
pub const fn usage() -> &'static str {
    "Usage:\n  pastral-agent health-check --data-root <path>\n  pastral-agent capture-current --data-root <path>\n  pastral-agent listen --data-root <path> [--max-events <positive-integer>]"
}
