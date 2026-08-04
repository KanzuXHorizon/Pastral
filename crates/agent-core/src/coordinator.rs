use core::num::NonZeroUsize;
use std::time::Duration;

use pastral_domain::{CaptureOrder, ClipEventId, ProfileId, ProtectionDomain};

use crate::{
    AgentError, CaptureSequence, CaptureSink, CaptureSource, CaptureSourceError, Clock, Sleeper,
    TextCaptureRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    profile_id: ProfileId,
    protection_domain: ProtectionDomain,
    max_unicode_text_bytes: NonZeroUsize,
    retry_delays: Vec<Duration>,
}

impl CaptureConfig {
    pub fn new(
        profile_id: ProfileId,
        protection_domain: ProtectionDomain,
        max_unicode_text_bytes: NonZeroUsize,
        retry_delays: Vec<Duration>,
    ) -> Result<Self, AgentError> {
        if !matches!(protection_domain, ProtectionDomain::Ordinary(_)) {
            return Err(AgentError::NonOrdinaryProtectionDomain);
        }
        if retry_delays.is_empty() {
            return Err(AgentError::InvalidRetrySchedule("empty"));
        }
        if retry_delays[0] != Duration::ZERO {
            return Err(AgentError::InvalidRetrySchedule(
                "first attempt must be immediate",
            ));
        }
        Ok(Self {
            profile_id,
            protection_domain,
            max_unicode_text_bytes,
            retry_delays,
        })
    }

    #[must_use]
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn protection_domain(&self) -> ProtectionDomain {
        self.protection_domain
    }

    #[must_use]
    pub const fn max_unicode_text_bytes(&self) -> NonZeroUsize {
        self.max_unicode_text_bytes
    }

    #[must_use]
    pub fn retry_delays(&self) -> &[Duration] {
        &self.retry_delays
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOutcome {
    Stored {
        clip_event_id: ClipEventId,
        capture_order: CaptureOrder,
    },
    DuplicateNotification,
    NoSupportedRepresentation,
    RetryExhausted {
        attempts: usize,
    },
    InvalidCapture,
    PlatformFailure,
    StorageFailed,
}

pub struct CaptureCoordinator {
    config: CaptureConfig,
    last_handled_sequence: Option<CaptureSequence>,
}

impl CaptureCoordinator {
    pub fn new(config: CaptureConfig) -> Result<Self, AgentError> {
        Ok(Self {
            config,
            last_handled_sequence: None,
        })
    }

    pub fn handle_notification<S, K, C, L>(
        &mut self,
        sequence: CaptureSequence,
        source: &mut S,
        sink: &mut K,
        clock: &mut C,
        sleeper: &mut L,
    ) -> CaptureOutcome
    where
        S: CaptureSource,
        K: CaptureSink,
        C: Clock,
        L: Sleeper,
    {
        if self.last_handled_sequence == Some(sequence) {
            return CaptureOutcome::DuplicateNotification;
        }

        for (index, delay) in self.config.retry_delays.iter().copied().enumerate() {
            if delay != Duration::ZERO {
                sleeper.sleep(delay);
            }
            match source.capture_unicode_text(self.config.max_unicode_text_bytes) {
                Ok(Some(captured_text)) => {
                    let observed_at = match clock.now_utc_micros() {
                        Ok(value) => value,
                        Err(_) => {
                            self.last_handled_sequence = Some(sequence);
                            return CaptureOutcome::PlatformFailure;
                        }
                    };
                    let request = TextCaptureRequest::new(
                        observed_at,
                        self.config.profile_id,
                        self.config.protection_domain,
                        captured_text,
                    );
                    return match sink.store_text(request) {
                        Ok(stored) => {
                            self.last_handled_sequence = Some(sequence);
                            CaptureOutcome::Stored {
                                clip_event_id: stored.clip_event_id(),
                                capture_order: stored.capture_order(),
                            }
                        }
                        Err(_) => CaptureOutcome::StorageFailed,
                    };
                }
                Ok(None) => {
                    self.last_handled_sequence = Some(sequence);
                    return CaptureOutcome::NoSupportedRepresentation;
                }
                Err(CaptureSourceError::Busy) => {
                    if index + 1 == self.config.retry_delays.len() {
                        self.last_handled_sequence = Some(sequence);
                        return CaptureOutcome::RetryExhausted {
                            attempts: self.config.retry_delays.len(),
                        };
                    }
                }
                Err(CaptureSourceError::InvalidData) => {
                    self.last_handled_sequence = Some(sequence);
                    return CaptureOutcome::InvalidCapture;
                }
                Err(CaptureSourceError::PlatformFailure) => {
                    self.last_handled_sequence = Some(sequence);
                    return CaptureOutcome::PlatformFailure;
                }
            }
        }

        unreachable!("validated retry schedule always has at least one attempt")
    }
}
