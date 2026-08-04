use core::fmt;
use std::io::ErrorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRuntimeError {
    Io {
        operation: &'static str,
        kind: ErrorKind,
    },
    InvalidIdentity(&'static str),
    InvalidPrivacyPolicy(&'static str),
    Storage(&'static str),
    Clipboard(&'static str),
    CoordinatorConfiguration,
    NotificationChannelClosed,
    IntegrityFailed,
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
            Self::InvalidPrivacyPolicy(reason) => {
                write!(f, "agent privacy policy is invalid: {reason}")
            }
            Self::Storage(operation) => write!(f, "agent storage operation failed: {operation}"),
            Self::Clipboard(operation) => {
                write!(f, "agent clipboard operation failed: {operation}")
            }
            Self::CoordinatorConfiguration => write!(f, "capture coordinator configuration failed"),
            Self::NotificationChannelClosed => {
                write!(f, "clipboard notification channel closed")
            }
            Self::IntegrityFailed => write!(f, "storage integrity verification failed"),
        }
    }
}

impl std::error::Error for AgentRuntimeError {}
