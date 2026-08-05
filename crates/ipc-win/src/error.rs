use std::io;

use pastral_ipc_auth::AuthError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    Windows {
        operation: &'static str,
        code: u32,
    },
    NtStatus {
        operation: &'static str,
        status: i32,
    },
    InvalidIdentity(&'static str),
    InvalidSecretEnvelope(&'static str),
    InvalidTokenIdentity(&'static str),
    InvalidProcessMemory(&'static str),
    InvalidPipeName(&'static str),
    InvalidInstanceName(&'static str),
    Authentication(AuthError),
    Timeout(&'static str),
    Disconnected,
    Protocol(&'static str),
    SizeLimit(&'static str),
}

impl TransportError {
    pub(crate) fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io { operation, kind } => write!(formatter, "{operation} failed: {kind:?}"),
            Self::Windows { operation, code } => {
                write!(formatter, "{operation} failed with Win32 error {code}")
            }
            Self::NtStatus { operation, status } => {
                write!(formatter, "{operation} failed with NTSTATUS {status:#x}")
            }
            Self::InvalidIdentity(reason) => {
                write!(formatter, "invalid transport identity: {reason}")
            }
            Self::InvalidSecretEnvelope(reason) => {
                write!(formatter, "invalid secret envelope: {reason}")
            }
            Self::InvalidTokenIdentity(reason) => {
                write!(formatter, "invalid token identity: {reason}")
            }
            Self::InvalidProcessMemory(reason) => {
                write!(formatter, "invalid process memory evidence: {reason}")
            }
            Self::InvalidPipeName(reason) => write!(formatter, "invalid pipe name: {reason}"),
            Self::InvalidInstanceName(reason) => {
                write!(formatter, "invalid process instance name: {reason}")
            }
            Self::Authentication(error) => write!(formatter, "IPC authentication failed: {error}"),
            Self::Timeout(operation) => write!(formatter, "{operation} timed out"),
            Self::Disconnected => formatter.write_str("pipe disconnected"),
            Self::Protocol(reason) => write!(formatter, "pipe protocol error: {reason}"),
            Self::SizeLimit(reason) => write!(formatter, "transport size limit: {reason}"),
        }
    }
}

impl std::error::Error for TransportError {}
