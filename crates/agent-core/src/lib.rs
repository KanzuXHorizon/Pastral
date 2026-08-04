#![forbid(unsafe_code)]

mod capture;
mod coordinator;
mod error;
mod ports;
mod privacy;

pub use capture::{CaptureSequence, CapturedText, StoredCapture, TextCaptureRequest};
pub use coordinator::{CaptureConfig, CaptureCoordinator, CaptureOutcome};
pub use error::AgentError;
pub use ports::{
    CaptureSink, CaptureSinkError, CaptureSinkOutcome, CaptureSource, CaptureSourceError, Clock,
    Sleeper,
};
pub use privacy::{
    MAX_SECRET_SCAN_BYTES, SensitiveClass, SourceAdmissionDecision, SourceAdmissionPolicy,
    SourceConfidence, SourceObservation, detect_high_confidence_secret,
};
