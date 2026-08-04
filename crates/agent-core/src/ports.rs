use core::num::NonZeroUsize;
use std::time::Duration;

use pastral_domain::UtcUnixMicros;

use crate::{AgentError, CapturedText, StoredCapture, TextCaptureRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSourceError {
    Busy,
    HardDenied,
    PolicyDenied,
    InvalidData,
    PlatformFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSinkError {
    StorageFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSinkOutcome {
    Stored(StoredCapture),
    SensitiveSkipped,
}

pub trait CaptureSource {
    fn capture_unicode_text(
        &mut self,
        max_bytes: NonZeroUsize,
    ) -> Result<Option<CapturedText>, CaptureSourceError>;
}

pub trait CaptureSink {
    fn store_text(
        &mut self,
        request: TextCaptureRequest,
    ) -> Result<CaptureSinkOutcome, CaptureSinkError>;
}

pub trait Clock {
    fn now_utc_micros(&mut self) -> Result<UtcUnixMicros, AgentError>;
}

pub trait Sleeper {
    fn sleep(&mut self, duration: Duration);
}
