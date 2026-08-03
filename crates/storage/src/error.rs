use core::fmt;
use std::io;

use pastral_domain::{ClipEventId, ClipRepresentationId};

#[derive(Debug)]
pub enum StorageError {
    InvalidStorageLimit(&'static str),
    InvalidStoragePolicyVersion,
    UnsupportedSchemaVersion(i64),
    RequiredPragmaMismatch(&'static str),
    Fts5Unavailable,
    PayloadLimitExceeded(ClipRepresentationId),
    PayloadMappingMissing(ClipRepresentationId),
    PayloadMappingDuplicate(ClipRepresentationId),
    PayloadMappingUnknown(ClipRepresentationId),
    PayloadLengthMismatch(ClipRepresentationId),
    PayloadDigestMismatch(ClipRepresentationId),
    ProtectedPayloadRequiresEncryption(ClipRepresentationId),
    SearchProjectionInvalid(&'static str),
    SearchQueryInvalid(&'static str),
    DuplicateClipEvent(ClipEventId),
    DuplicateRepresentation(ClipRepresentationId),
    DuplicateCaptureOrder,
    UnsafeExternalLocator,
    BlobMissing,
    BlobIntegrityMismatch,
    BlobLengthMismatch,
    IntegerOutOfRange(&'static str),
    Domain(String),
    Database(rusqlite::Error),
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
}

impl StorageError {
    pub(crate) fn io(operation: &'static str, source: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: source.kind(),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStorageLimit(name) => write!(f, "storage limit must be non-zero: {name}"),
            Self::InvalidStoragePolicyVersion => {
                write!(f, "storage policy version must be non-zero")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    f,
                    "database schema version {version} is newer than supported"
                )
            }
            Self::RequiredPragmaMismatch(name) => {
                write!(f, "required SQLite pragma mismatch: {name}")
            }
            Self::Fts5Unavailable => write!(f, "SQLite FTS5 support is unavailable"),
            Self::PayloadLimitExceeded(id) => write!(
                f,
                "payload exceeds configured limit for representation {id}"
            ),
            Self::PayloadMappingMissing(id) => {
                write!(f, "payload is missing for representation {id}")
            }
            Self::PayloadMappingDuplicate(id) => {
                write!(f, "payload is duplicated for representation {id}")
            }
            Self::PayloadMappingUnknown(id) => {
                write!(f, "payload references unknown representation {id}")
            }
            Self::PayloadLengthMismatch(id) => {
                write!(f, "payload length mismatch for representation {id}")
            }
            Self::PayloadDigestMismatch(id) => {
                write!(f, "payload digest mismatch for representation {id}")
            }
            Self::ProtectedPayloadRequiresEncryption(id) => {
                write!(
                    f,
                    "protected payload requires encryption for representation {id}"
                )
            }
            Self::SearchProjectionInvalid(reason) => {
                write!(f, "search projection is invalid: {reason}")
            }
            Self::SearchQueryInvalid(reason) => write!(f, "search query is invalid: {reason}"),
            Self::DuplicateClipEvent(id) => write!(f, "clip event already exists: {id}"),
            Self::DuplicateRepresentation(id) => {
                write!(f, "clip representation already exists: {id}")
            }
            Self::DuplicateCaptureOrder => write!(f, "capture order already exists"),
            Self::UnsafeExternalLocator => write!(f, "external blob locator is unsafe"),
            Self::BlobMissing => write!(f, "blob payload is missing"),
            Self::BlobIntegrityMismatch => write!(f, "blob integrity verification failed"),
            Self::BlobLengthMismatch => write!(f, "blob length verification failed"),
            Self::IntegerOutOfRange(name) => {
                write!(f, "integer value is outside supported range: {name}")
            }
            Self::Domain(message) => write!(f, "domain reconstruction failed: {message}"),
            Self::Database(source) => write!(f, "SQLite operation failed: {source}"),
            Self::Io { operation, kind } => {
                write!(f, "I/O operation failed ({operation}): {kind:?}")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}
