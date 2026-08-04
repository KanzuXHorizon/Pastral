use core::num::NonZeroU32;

use pastral_domain::{CaptureOrder, ClipEventId, ProfileId, ProtectionDomain, UtcUnixMicros};

use crate::AgentError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaptureSequence(NonZeroU32);

impl CaptureSequence {
    pub fn new(value: u32) -> Result<Self, AgentError> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(AgentError::InvalidSequence)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedText {
    text: String,
    raw_utf16le: Vec<u8>,
}

impl CapturedText {
    pub fn new(text: String, raw_utf16le: Vec<u8>) -> Result<Self, AgentError> {
        if raw_utf16le.len() < 2 {
            return Err(AgentError::InvalidCapturedText("missing terminator"));
        }
        if !raw_utf16le.len().is_multiple_of(2) {
            return Err(AgentError::InvalidCapturedText("misaligned UTF-16LE bytes"));
        }

        let mut units = Vec::with_capacity(raw_utf16le.len() / 2);
        let mut terminator_offset = None;
        for (index, pair) in raw_utf16le.chunks_exact(2).enumerate() {
            let unit = u16::from_le_bytes([pair[0], pair[1]]);
            if unit == 0 {
                terminator_offset = Some((index + 1) * 2);
                break;
            }
            units.push(unit);
        }
        let terminator_offset =
            terminator_offset.ok_or(AgentError::InvalidCapturedText("missing terminator"))?;
        if terminator_offset != raw_utf16le.len() {
            return Err(AgentError::InvalidCapturedText(
                "bytes continue after first terminator",
            ));
        }
        let decoded = String::from_utf16(&units)
            .map_err(|_| AgentError::InvalidCapturedText("invalid UTF-16"))?;
        if decoded != text {
            return Err(AgentError::InvalidCapturedText(
                "decoded text does not match exact bytes",
            ));
        }

        Ok(Self { text, raw_utf16le })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn raw_utf16le(&self) -> &[u8] {
        &self.raw_utf16le
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextCaptureRequest {
    observed_at: UtcUnixMicros,
    profile_id: ProfileId,
    protection_domain: ProtectionDomain,
    captured_text: CapturedText,
}

impl TextCaptureRequest {
    #[must_use]
    pub fn new(
        observed_at: UtcUnixMicros,
        profile_id: ProfileId,
        protection_domain: ProtectionDomain,
        captured_text: CapturedText,
    ) -> Self {
        Self {
            observed_at,
            profile_id,
            protection_domain,
            captured_text,
        }
    }

    #[must_use]
    pub const fn observed_at(&self) -> UtcUnixMicros {
        self.observed_at
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
    pub const fn captured_text(&self) -> &CapturedText {
        &self.captured_text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredCapture {
    clip_event_id: ClipEventId,
    capture_order: CaptureOrder,
}

impl StoredCapture {
    #[must_use]
    pub const fn new(clip_event_id: ClipEventId, capture_order: CaptureOrder) -> Self {
        Self {
            clip_event_id,
            capture_order,
        }
    }

    #[must_use]
    pub const fn clip_event_id(self) -> ClipEventId {
        self.clip_event_id
    }

    #[must_use]
    pub const fn capture_order(self) -> CaptureOrder {
        self.capture_order
    }
}
