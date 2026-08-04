use core::fmt;

use crate::FrameKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    InvalidLimit(&'static str),
    InvalidMagic,
    UnsupportedFramingMajor(u16),
    UnknownFrameKind(u16),
    ReservedFlags(u32),
    BodyLengthExceeded {
        kind: FrameKind,
        length: u32,
        limit: u32,
    },
    EmptyBulkChunk,
    FrameBodyLengthMismatch {
        declared: u32,
        actual: usize,
    },
    InvalidFrameSequence,
    InvalidCorrelation,
    DecoderPoisoned,
    TruncatedFrame,
    TooManyFrames,
    InvalidConnectionState,
    DuplicateCorrelation,
    InFlightLimitExceeded,
    BulkNotAuthorized,
    BulkSequenceMismatch {
        expected: u32,
        actual: u32,
    },
    BulkLengthExceeded,
    BulkChunkLimitExceeded,
    BulkEndMismatch,
    IntegerOverflow,
    InvalidDto(&'static str),
    Schema(String),
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(f, "IPC limit must be nonzero: {name}"),
            Self::InvalidMagic => write!(f, "IPC frame magic is invalid"),
            Self::UnsupportedFramingMajor(value) => {
                write!(f, "unsupported IPC framing major: {value}")
            }
            Self::UnknownFrameKind(value) => write!(f, "unknown IPC frame kind: {value}"),
            Self::ReservedFlags(value) => write!(f, "IPC frame has reserved flags: {value}"),
            Self::BodyLengthExceeded {
                kind,
                length,
                limit,
            } => write!(
                f,
                "IPC frame body length {length} exceeds {limit} for {kind:?}"
            ),
            Self::EmptyBulkChunk => write!(f, "IPC bulk chunk body must not be empty"),
            Self::FrameBodyLengthMismatch { declared, actual } => write!(
                f,
                "IPC frame body length mismatch: header declares {declared}, body has {actual} bytes"
            ),
            Self::InvalidFrameSequence => write!(f, "IPC frame sequence is invalid for its kind"),
            Self::InvalidCorrelation => write!(f, "IPC correlation identifier is invalid"),
            Self::DecoderPoisoned => write!(f, "IPC decoder is poisoned"),
            Self::TruncatedFrame => write!(f, "IPC byte stream ended with a truncated frame"),
            Self::TooManyFrames => write!(f, "IPC input batch contains too many frames"),
            Self::InvalidConnectionState => write!(f, "IPC frame is invalid for connection state"),
            Self::DuplicateCorrelation => write!(f, "IPC correlation is already in flight"),
            Self::InFlightLimitExceeded => write!(f, "IPC in-flight request limit exceeded"),
            Self::BulkNotAuthorized => write!(f, "IPC bulk transfer is not authorized"),
            Self::BulkSequenceMismatch { expected, actual } => write!(
                f,
                "IPC bulk sequence mismatch: expected {expected}, received {actual}"
            ),
            Self::BulkLengthExceeded => write!(f, "IPC bulk transfer byte limit exceeded"),
            Self::BulkChunkLimitExceeded => write!(f, "IPC bulk transfer chunk limit exceeded"),
            Self::BulkEndMismatch => write!(f, "IPC bulk end metadata does not match progress"),
            Self::IntegerOverflow => write!(f, "IPC integer arithmetic overflowed"),
            Self::InvalidDto(reason) => write!(f, "IPC DTO is invalid: {reason}"),
            Self::Schema(reason) => write!(f, "IPC schema operation failed: {reason}"),
        }
    }
}

impl std::error::Error for IpcError {}
