use uuid::Uuid;

use crate::{FrameLimits, IpcError};

pub const FRAME_HEADER_BYTES: usize = 36;
pub const FRAMING_MAJOR: u16 = 1;
const MAGIC: [u8; 4] = *b"PSTR";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum FrameKind {
    HelloProto = 1,
    ControlProto = 2,
    BulkChunk = 3,
    BulkEndProto = 4,
    ProtocolErrorProto = 5,
}

impl FrameKind {
    pub const fn from_raw(value: u16) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::HelloProto),
            2 => Ok(Self::ControlProto),
            3 => Ok(Self::BulkChunk),
            4 => Ok(Self::BulkEndProto),
            5 => Ok(Self::ProtocolErrorProto),
            _ => Err(IpcError::UnknownFrameKind(value)),
        }
    }

    #[must_use]
    pub const fn as_raw(self) -> u16 {
        self as u16
    }

    #[must_use]
    pub const fn uses_control_limit(self) -> bool {
        !matches!(self, Self::BulkChunk)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorrelationId([u8; 16]);

impl CorrelationId {
    #[must_use]
    pub const fn initial_server_hello() -> Self {
        Self([0; 16])
    }

    #[must_use]
    pub fn new_v4() -> Self {
        Self(*Uuid::new_v4().as_bytes())
    }

    pub const fn from_bytes(bytes: [u8; 16]) -> Result<Self, IpcError> {
        if is_zero_bytes(&bytes) || is_uuid_v4_rfc4122(&bytes) {
            Ok(Self(bytes))
        } else {
            Err(IpcError::InvalidCorrelation)
        }
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        is_zero_bytes(&self.0)
    }
}

impl core::fmt::Debug for CorrelationId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_zero() {
            f.write_str("CorrelationId(INITIAL_SERVER_HELLO)")
        } else {
            f.write_str("CorrelationId(UUIDv4)")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    kind: FrameKind,
    body_length: u32,
    sequence: u32,
    correlation: CorrelationId,
}

impl FrameHeader {
    pub fn new(
        kind: FrameKind,
        body_length: u32,
        sequence: u32,
        correlation: CorrelationId,
        limits: FrameLimits,
    ) -> Result<Self, IpcError> {
        validate_body_length(kind, body_length, limits)?;
        validate_sequence(kind, sequence)?;
        validate_correlation(kind, correlation)?;
        Ok(Self {
            kind,
            body_length,
            sequence,
            correlation,
        })
    }

    pub fn decode(bytes: &[u8; FRAME_HEADER_BYTES], limits: FrameLimits) -> Result<Self, IpcError> {
        if bytes[0..4] != MAGIC {
            return Err(IpcError::InvalidMagic);
        }
        let framing_major = u16::from_le_bytes([bytes[4], bytes[5]]);
        if framing_major != FRAMING_MAJOR {
            return Err(IpcError::UnsupportedFramingMajor(framing_major));
        }
        let kind = FrameKind::from_raw(u16::from_le_bytes([bytes[6], bytes[7]]))?;
        let flags = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if flags != 0 {
            return Err(IpcError::ReservedFlags(flags));
        }
        let body_length = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let sequence = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let mut correlation_bytes = [0u8; 16];
        correlation_bytes.copy_from_slice(&bytes[20..36]);
        let correlation = CorrelationId::from_bytes(correlation_bytes)?;
        Self::new(kind, body_length, sequence, correlation, limits)
    }

    #[must_use]
    pub fn encode(self) -> [u8; FRAME_HEADER_BYTES] {
        let mut bytes = [0u8; FRAME_HEADER_BYTES];
        bytes[0..4].copy_from_slice(&MAGIC);
        bytes[4..6].copy_from_slice(&FRAMING_MAJOR.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.kind.as_raw().to_le_bytes());
        bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.body_length.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[20..36].copy_from_slice(self.correlation.as_bytes());
        bytes
    }

    #[must_use]
    pub const fn kind(self) -> FrameKind {
        self.kind
    }

    #[must_use]
    pub const fn body_length(self) -> u32 {
        self.body_length
    }

    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.sequence
    }

    #[must_use]
    pub const fn correlation(self) -> CorrelationId {
        self.correlation
    }
}

const fn validate_body_length(
    kind: FrameKind,
    body_length: u32,
    limits: FrameLimits,
) -> Result<(), IpcError> {
    let limit = if kind.uses_control_limit() {
        limits.max_control_body_bytes()
    } else {
        limits.max_bulk_chunk_bytes()
    };
    if body_length > limit {
        return Err(IpcError::BodyLengthExceeded {
            kind,
            length: body_length,
            limit,
        });
    }
    if matches!(kind, FrameKind::BulkChunk) && body_length == 0 {
        return Err(IpcError::EmptyBulkChunk);
    }
    Ok(())
}

const fn validate_sequence(kind: FrameKind, sequence: u32) -> Result<(), IpcError> {
    if matches!(
        kind,
        FrameKind::HelloProto | FrameKind::ControlProto | FrameKind::ProtocolErrorProto
    ) && sequence != 0
    {
        return Err(IpcError::InvalidFrameSequence);
    }
    Ok(())
}

const fn validate_correlation(kind: FrameKind, correlation: CorrelationId) -> Result<(), IpcError> {
    if correlation.is_zero() && !matches!(kind, FrameKind::HelloProto) {
        return Err(IpcError::InvalidCorrelation);
    }
    Ok(())
}

const fn is_zero_bytes(bytes: &[u8; 16]) -> bool {
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != 0 {
            return false;
        }
        index += 1;
    }
    true
}

const fn is_uuid_v4_rfc4122(bytes: &[u8; 16]) -> bool {
    (bytes[6] >> 4) == 4 && (bytes[8] & 0b1100_0000) == 0b1000_0000
}
