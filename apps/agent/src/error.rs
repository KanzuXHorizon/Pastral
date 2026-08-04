use core::fmt;
use std::io::ErrorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRuntimeError {
    Io {
        operation: &'static str,
        kind: ErrorKind,
    },
    InvalidIdentity(&'static str),
}

impl AgentRuntimeError {
    #[must_use]
    pub fn io(operation: &'static str, error: &std::io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

impl fmt::Display for AgentRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, kind } => {
                write!(
                    f,
                    "agent filesystem operation failed ({operation}, {kind:?})"
                )
            }
            Self::InvalidIdentity(reason) => write!(f, "agent identity is invalid: {reason}"),
        }
    }
}

impl std::error::Error for AgentRuntimeError {}
