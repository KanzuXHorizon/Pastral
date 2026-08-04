use core::num::NonZeroUsize;

use pastral_agent_core::{CaptureSource, CaptureSourceError, CapturedText};
use pastral_clipboard_win::{ClipboardError, ClipboardSession};

#[derive(Debug, Default)]
pub struct WindowsClipboardSource;

impl CaptureSource for WindowsClipboardSource {
    fn capture_unicode_text(
        &mut self,
        max_bytes: NonZeroUsize,
    ) -> Result<Option<CapturedText>, CaptureSourceError> {
        let session = ClipboardSession::open_read().map_err(map_clipboard_error)?;
        let captured = session
            .capture_unicode_text(max_bytes)
            .map_err(map_clipboard_error)?;
        captured
            .map(|value| {
                CapturedText::new(value.text().to_owned(), value.raw_logical_bytes().to_vec())
                    .map_err(|_| CaptureSourceError::InvalidData)
            })
            .transpose()
    }
}

fn map_clipboard_error(error: ClipboardError) -> CaptureSourceError {
    match error {
        ClipboardError::ClipboardUnavailable => CaptureSourceError::Busy,
        ClipboardError::HistoryControlInvalid => CaptureSourceError::HardDenied,
        ClipboardError::DataHandleUnavailable
        | ClipboardError::GlobalAllocationEmpty
        | ClipboardError::GlobalAllocationTooLarge { .. }
        | ClipboardError::GlobalLockFailed
        | ClipboardError::GlobalUnlockFailed
        | ClipboardError::UnicodeTextMissingTerminator
        | ClipboardError::UnicodeTextMisaligned
        | ClipboardError::UnicodeTextInvalidUtf16
        | ClipboardError::LengthOutOfRange => CaptureSourceError::InvalidData,
        ClipboardError::Win32 {
            operation: "OpenClipboard",
            ..
        } => CaptureSourceError::Busy,
        ClipboardError::InvalidRuntimeFormatId
        | ClipboardError::ListenerThreadSpawn
        | ClipboardError::ListenerStartupClosed
        | ClipboardError::ListenerThreadPanicked
        | ClipboardError::Win32 { .. }
        | ClipboardError::FormatLimitExceeded { .. }
        | ClipboardError::RegisteredNameTooLong
        | ClipboardError::RegisteredNameInvalidUtf16
        | ClipboardError::RegisteredNameInvalid => CaptureSourceError::PlatformFailure,
    }
}
