use core::num::NonZeroUsize;

use crate::{ClipboardError, sys};

pub(crate) fn copy_clipboard_global(
    handle: sys::NativeGlobalHandle,
    max_bytes: NonZeroUsize,
) -> Result<Vec<u8>, ClipboardError> {
    sys::copy_global(handle, max_bytes.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_global_copies_exact_bytes_and_enforces_bound() {
        let bytes = [0x00, 0x01, 0xFE, 0xFF];
        assert_eq!(
            sys::copy_synthetic_global(&bytes, bytes.len()).unwrap(),
            bytes
        );
        assert_eq!(
            sys::copy_synthetic_global(&bytes, bytes.len() - 1),
            Err(ClipboardError::GlobalAllocationTooLarge {
                size: bytes.len(),
                limit: bytes.len() - 1,
            })
        );
    }

    #[test]
    fn zero_sized_global_is_rejected() {
        assert!(matches!(
            sys::copy_synthetic_global(&[], 1),
            Err(ClipboardError::GlobalAllocationEmpty)
                | Err(ClipboardError::Win32 {
                    operation: "GlobalAlloc",
                    ..
                })
        ));
    }
}
