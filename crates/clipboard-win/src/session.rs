use core::{marker::PhantomData, num::NonZeroUsize};
use std::rc::Rc;

use crate::{
    CapturedUnicodeText, ClipboardError, ClipboardFormatDescriptor, ClipboardHistoryControls,
    ClipboardOwnerObservation,
    enumeration::{Win32FormatSource, enumerate_formats},
    format::CF_UNICODETEXT_ID,
    hglobal::copy_clipboard_global,
    history_controls::inspect_history_controls,
    sys,
};

pub struct ClipboardSession {
    open: bool,
    _thread_bound: PhantomData<Rc<()>>,
}

impl ClipboardSession {
    pub fn open_read() -> Result<Self, ClipboardError> {
        sys::open_clipboard()?;
        Ok(Self {
            open: true,
            _thread_bound: PhantomData,
        })
    }

    pub fn enumerate_formats(
        &self,
        max_formats: NonZeroUsize,
    ) -> Result<Vec<ClipboardFormatDescriptor>, ClipboardError> {
        let mut source = Win32FormatSource;
        enumerate_formats(&mut source, max_formats)
    }

    pub fn history_controls(&self) -> Result<ClipboardHistoryControls, ClipboardError> {
        inspect_history_controls()
    }

    pub fn owner_process(&self) -> Result<ClipboardOwnerObservation, ClipboardError> {
        ClipboardOwnerObservation::observe()
    }

    pub fn capture_unicode_text(
        &self,
        max_bytes: NonZeroUsize,
    ) -> Result<Option<CapturedUnicodeText>, ClipboardError> {
        if !sys::is_format_available(CF_UNICODETEXT_ID) {
            return Ok(None);
        }
        let handle = sys::clipboard_data_handle(CF_UNICODETEXT_ID)?;
        let bytes = copy_clipboard_global(handle, max_bytes)?;
        CapturedUnicodeText::parse(&bytes).map(Some)
    }

    pub fn close(mut self) -> Result<(), ClipboardError> {
        if self.open {
            sys::close_clipboard()?;
            self.open = false;
        }
        Ok(())
    }
}

impl Drop for ClipboardSession {
    fn drop(&mut self) {
        if self.open {
            let _ = sys::close_clipboard();
            self.open = false;
        }
    }
}
