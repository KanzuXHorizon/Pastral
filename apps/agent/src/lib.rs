#![cfg(windows)]
#![forbid(unsafe_code)]

mod clock;
mod config;
mod error;
mod platform;
mod storage_sink;

pub use clock::{SystemClock, ThreadSleeper};
pub use config::AgentIdentity;
pub use error::AgentRuntimeError;
pub use platform::WindowsClipboardSource;
pub use storage_sink::{DiagnosticStoragePolicy, StorageCaptureSink, diagnostic_storage_limits};
