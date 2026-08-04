#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod enumeration;
mod error;
mod format;
mod hglobal;
mod history_controls;
mod listener;
mod owner;
mod sequence;
mod session;
mod sys;
mod unicode_text;

pub use error::{ClipboardError, NotificationReceiveError};
pub use format::{ClipboardFormatDescriptor, RuntimeClipboardFormatId, RuntimeFormatKind};
pub use history_controls::ClipboardHistoryControls;
pub use listener::{ClipboardListener, ClipboardNotifications};
pub use owner::ClipboardOwnerObservation;
pub use sequence::{ClipboardNotification, ClipboardSequence};
pub use session::ClipboardSession;
pub use unicode_text::CapturedUnicodeText;
