use core::num::NonZeroUsize;

use crate::{ClipboardError, hglobal::copy_clipboard_global, sys};

pub(crate) const EXCLUDE_MONITOR_PROCESSING_FORMAT: &str =
    "ExcludeClipboardContentFromMonitorProcessing";
pub(crate) const INCLUDE_IN_HISTORY_FORMAT: &str = "CanIncludeInClipboardHistory";
pub(crate) const UPLOAD_TO_CLOUD_FORMAT: &str = "CanUploadToCloudClipboard";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardHistoryControls {
    exclude_from_monitor_processing: bool,
    include_in_history: Option<bool>,
    upload_to_cloud: Option<bool>,
}

impl ClipboardHistoryControls {
    #[must_use]
    pub const fn new(
        exclude_from_monitor_processing: bool,
        include_in_history: Option<bool>,
        upload_to_cloud: Option<bool>,
    ) -> Self {
        Self {
            exclude_from_monitor_processing,
            include_in_history,
            upload_to_cloud,
        }
    }

    #[must_use]
    pub const fn exclude_from_monitor_processing(self) -> bool {
        self.exclude_from_monitor_processing
    }

    #[must_use]
    pub const fn include_in_history(self) -> Option<bool> {
        self.include_in_history
    }

    #[must_use]
    pub const fn upload_to_cloud(self) -> Option<bool> {
        self.upload_to_cloud
    }

    #[must_use]
    pub const fn is_hard_denied(self) -> bool {
        self.exclude_from_monitor_processing || matches!(self.include_in_history, Some(false))
    }
}

pub(crate) fn inspect_history_controls() -> Result<ClipboardHistoryControls, ClipboardError> {
    let exclude_id = sys::register_clipboard_format(EXCLUDE_MONITOR_PROCESSING_FORMAT)?;
    let include_id = sys::register_clipboard_format(INCLUDE_IN_HISTORY_FORMAT)?;
    let upload_id = sys::register_clipboard_format(UPLOAD_TO_CLOUD_FORMAT)?;

    Ok(ClipboardHistoryControls::new(
        sys::is_format_available(exclude_id),
        read_optional_control(include_id)?,
        read_optional_control(upload_id)?,
    ))
}

fn read_optional_control(format_id: u32) -> Result<Option<bool>, ClipboardError> {
    if !sys::is_format_available(format_id) {
        return Ok(None);
    }
    let handle = sys::clipboard_data_handle(format_id)?;
    let bytes = copy_clipboard_global(
        handle,
        NonZeroUsize::new(size_of::<u32>()).expect("DWORD size is nonzero"),
    )?;
    parse_control_dword(&bytes).map(Some)
}

fn parse_control_dword(bytes: &[u8]) -> Result<bool, ClipboardError> {
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| ClipboardError::HistoryControlInvalid)?;
    match u32::from_le_bytes(bytes) {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ClipboardError::HistoryControlInvalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_controls_have_no_effect() {
        let controls = ClipboardHistoryControls::new(false, None, None);
        assert!(!controls.is_hard_denied());
        assert_eq!(controls.include_in_history(), None);
        assert_eq!(controls.upload_to_cloud(), None);
    }

    #[test]
    fn exclude_or_history_false_is_hard_deny() {
        assert!(ClipboardHistoryControls::new(true, None, None).is_hard_denied());
        assert!(ClipboardHistoryControls::new(false, Some(false), None).is_hard_denied());
        assert!(!ClipboardHistoryControls::new(false, Some(true), Some(false)).is_hard_denied());
    }

    #[test]
    fn dword_parser_accepts_only_exact_zero_or_one() {
        assert_eq!(parse_control_dword(&0_u32.to_le_bytes()), Ok(false));
        assert_eq!(parse_control_dword(&1_u32.to_le_bytes()), Ok(true));
        assert_eq!(
            parse_control_dword(&[0, 0, 0]),
            Err(ClipboardError::HistoryControlInvalid)
        );
        assert_eq!(
            parse_control_dword(&2_u32.to_le_bytes()),
            Err(ClipboardError::HistoryControlInvalid)
        );
    }
}
