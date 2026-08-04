use core::num::NonZeroUsize;

use pastral_agent_core::{
    CaptureSource, CaptureSourceError, CapturedText, SourceAdmissionDecision,
    SourceAdmissionPolicy, SourceObservation,
};
use pastral_clipboard_win::{
    ClipboardError, ClipboardHistoryControls, ClipboardOwnerObservation, ClipboardSession,
};

#[derive(Debug, Clone)]
pub struct WindowsClipboardSource {
    policy: SourceAdmissionPolicy,
}

impl WindowsClipboardSource {
    #[must_use]
    pub const fn new(policy: SourceAdmissionPolicy) -> Self {
        Self { policy }
    }
}

impl CaptureSource for WindowsClipboardSource {
    fn capture_unicode_text(
        &mut self,
        max_bytes: NonZeroUsize,
    ) -> Result<Option<CapturedText>, CaptureSourceError> {
        let session = ClipboardSession::open_read().map_err(map_clipboard_error)?;
        let controls = session.history_controls().map_err(map_clipboard_error)?;
        if controls.is_hard_denied() {
            return Err(CaptureSourceError::HardDenied);
        }
        let owner = session.owner_process().map_err(map_clipboard_error)?;
        evaluate_source_admission(controls, owner, &self.policy)?;
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

fn evaluate_source_admission(
    controls: ClipboardHistoryControls,
    owner: ClipboardOwnerObservation,
    policy: &SourceAdmissionPolicy,
) -> Result<(), CaptureSourceError> {
    if controls.is_hard_denied() {
        return Err(CaptureSourceError::HardDenied);
    }
    let observation = match owner {
        ClipboardOwnerObservation::Unavailable => SourceObservation::unavailable(),
        ClipboardOwnerObservation::ProcessImage(path) => path
            .file_name()
            .and_then(|value| value.to_str())
            .and_then(|value| SourceObservation::from_executable_name(value).ok())
            .unwrap_or_else(SourceObservation::unavailable),
    };
    match policy.evaluate(&observation) {
        SourceAdmissionDecision::Allow => Ok(()),
        SourceAdmissionDecision::DenyUnresolved | SourceAdmissionDecision::DenyExecutable => {
            Err(CaptureSourceError::PolicyDenied)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use pastral_agent_core::SourceAdmissionPolicy;
    use pastral_clipboard_win::{ClipboardHistoryControls, ClipboardOwnerObservation};
    use std::path::PathBuf;

    #[test]
    fn source_owned_hard_deny_precedes_owner_policy() {
        let policy = SourceAdmissionPolicy::new(false, Vec::<String>::new()).unwrap();
        assert_eq!(
            evaluate_source_admission(
                ClipboardHistoryControls::new(true, None, None),
                ClipboardOwnerObservation::ProcessImage(PathBuf::from("C:\\Windows\\notepad.exe")),
                &policy,
            ),
            Err(CaptureSourceError::HardDenied)
        );
    }

    #[test]
    fn unresolved_and_exact_denied_sources_fail_closed() {
        let policy = SourceAdmissionPolicy::new(true, ["KeePassXC.exe"]).unwrap();
        assert_eq!(
            evaluate_source_admission(
                ClipboardHistoryControls::new(false, None, None),
                ClipboardOwnerObservation::Unavailable,
                &policy,
            ),
            Err(CaptureSourceError::PolicyDenied)
        );
        assert_eq!(
            evaluate_source_admission(
                ClipboardHistoryControls::new(false, Some(true), Some(false)),
                ClipboardOwnerObservation::ProcessImage(PathBuf::from(
                    "C:\\Program Files\\KeePassXC\\keepassxc.exe",
                )),
                &policy,
            ),
            Err(CaptureSourceError::PolicyDenied)
        );
    }

    #[test]
    fn resolved_unlisted_source_is_allowed() {
        let policy = SourceAdmissionPolicy::new(true, ["keepassxc.exe"]).unwrap();
        assert_eq!(
            evaluate_source_admission(
                ClipboardHistoryControls::new(false, Some(true), Some(false)),
                ClipboardOwnerObservation::ProcessImage(PathBuf::from(
                    "C:\\Windows\\System32\\notepad.exe",
                )),
                &policy,
            ),
            Ok(())
        );
    }
}
