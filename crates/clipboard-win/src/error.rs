use core::fmt;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    InvalidRuntimeFormatId,
    ListenerThreadSpawn,
    ListenerStartupClosed,
    ListenerThreadPanicked,
    Win32 { operation: &'static str, code: u32 },
    ClipboardUnavailable,
    FormatLimitExceeded { limit: usize },
    RegisteredNameTooLong,
    RegisteredNameInvalidUtf16,
    RegisteredNameInvalid,
    DataHandleUnavailable,
    GlobalAllocationEmpty,
    GlobalAllocationTooLarge { size: usize, limit: usize },
    GlobalLockFailed,
    GlobalUnlockFailed,
    UnicodeTextMissingTerminator,
    UnicodeTextMisaligned,
    UnicodeTextInvalidUtf16,
    LengthOutOfRange,
}

impl ClipboardError {
    #[must_use]
    pub const fn win32(operation: &'static str, code: u32) -> Self {
        Self::Win32 { operation, code }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRuntimeFormatId => {
                write!(f, "runtime clipboard format ID must be nonzero")
            }
            Self::ListenerThreadSpawn => write!(f, "failed to spawn clipboard listener thread"),
            Self::ListenerStartupClosed => {
                write!(f, "clipboard listener closed before startup completed")
            }
            Self::ListenerThreadPanicked => write!(f, "clipboard listener thread panicked"),
            Self::Win32 { operation, code } => {
                write!(
                    f,
                    "Win32 clipboard operation failed ({operation}, code {code})"
                )
            }
            Self::ClipboardUnavailable => write!(f, "clipboard is unavailable"),
            Self::FormatLimitExceeded { limit } => {
                write!(f, "clipboard format count exceeds configured limit {limit}")
            }
            Self::RegisteredNameTooLong => {
                write!(f, "registered clipboard format name exceeds limit")
            }
            Self::RegisteredNameInvalidUtf16 => {
                write!(f, "registered clipboard format name is invalid UTF-16")
            }
            Self::RegisteredNameInvalid => write!(f, "registered clipboard format name is invalid"),
            Self::DataHandleUnavailable => write!(f, "clipboard data handle is unavailable"),
            Self::GlobalAllocationEmpty => write!(f, "global memory allocation has zero size"),
            Self::GlobalAllocationTooLarge { size, limit } => {
                write!(
                    f,
                    "global memory allocation size {size} exceeds limit {limit}"
                )
            }
            Self::GlobalLockFailed => write!(f, "global memory lock failed"),
            Self::GlobalUnlockFailed => write!(f, "global memory unlock failed"),
            Self::UnicodeTextMissingTerminator => {
                write!(f, "CF_UNICODETEXT payload has no aligned terminating NUL")
            }
            Self::UnicodeTextMisaligned => {
                write!(
                    f,
                    "CF_UNICODETEXT payload ends on a partial UTF-16 code unit"
                )
            }
            Self::UnicodeTextInvalidUtf16 => write!(f, "CF_UNICODETEXT payload is invalid UTF-16"),
            Self::LengthOutOfRange => write!(f, "native length is outside supported Rust range"),
        }
    }
}

impl std::error::Error for ClipboardError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationReceiveError {
    Empty,
    Timeout,
    Disconnected,
}

impl From<TryRecvError> for NotificationReceiveError {
    fn from(value: TryRecvError) -> Self {
        match value {
            TryRecvError::Empty => Self::Empty,
            TryRecvError::Disconnected => Self::Disconnected,
        }
    }
}

impl From<RecvTimeoutError> for NotificationReceiveError {
    fn from(value: RecvTimeoutError) -> Self {
        match value {
            RecvTimeoutError::Timeout => Self::Timeout,
            RecvTimeoutError::Disconnected => Self::Disconnected,
        }
    }
}
