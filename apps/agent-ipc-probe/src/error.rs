use core::fmt;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidArguments,
    AgentHealth,
    InvalidChildInput,
    Environment,
    Material,
    Process,
    Readiness,
    Transport,
    Authentication,
    Protocol,
    ChildFailure,
    MissingReleaseArtifact,
    InvalidMetric,
    FootprintCeiling,
    Cleanup,
    Timeout,
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
            Self::Environment => formatter.write_str("admission environment setup failed"),
            Self::Material => formatter.write_str("transport material failed"),
            Self::Process => formatter.write_str("child process operation failed"),
            Self::Readiness => formatter.write_str("child readiness failed"),
            Self::Transport => formatter.write_str("transport operation failed"),
            Self::Authentication => formatter.write_str("transport authentication failed"),
            Self::Protocol => formatter.write_str("Health protocol validation failed"),
            Self::ChildFailure => formatter.write_str("server child failed"),
            Self::MissingReleaseArtifact => formatter.write_str("required executable is missing"),
            Self::InvalidMetric => formatter.write_str("admission metric is invalid"),
            Self::FootprintCeiling => formatter.write_str("admission footprint ceiling failed"),
            Self::Cleanup => formatter.write_str("admission cleanup failed"),
            Self::Timeout => formatter.write_str("admission operation timed out"),
            Self::Io { operation, kind } => write!(formatter, "{operation} failed: {kind:?}"),
        }
    }
}

impl std::error::Error for AdmissionError {}
