#![forbid(unsafe_code)]

mod capture;
mod coordinator;
mod error;
mod ports;

pub use capture::{CaptureSequence, CapturedText, StoredCapture, TextCaptureRequest};
pub use coordinator::{CaptureConfig, CaptureCoordinator, CaptureOutcome};
pub use error::AgentError;
pub use ports::{CaptureSink, CaptureSinkError, CaptureSource, CaptureSourceError, Clock, Sleeper};
