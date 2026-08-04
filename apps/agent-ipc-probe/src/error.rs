use core::fmt;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidArguments,
    AgentHealth,
    InvalidChildInput,
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
}

impl AdmissionError {
    pub(crate) fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments => formatter.write_str("invalid arguments"),
            Self::AgentHealth => formatter.write_str("agent Health initialization failed"),
            Self::InvalidChildInput => formatter.write_str("child stdin contains command data"),
            Self::Io { operation, kind } => write!(formatter, "{operation} failed: {kind:?}"),
        }
    }
}

impl std::error::Error for AdmissionError {}
