use crate::IpcError;

pub const DEFAULT_MAX_CONTROL_BODY_BYTES: u32 = 256 * 1024;
pub const DEFAULT_MAX_BULK_CHUNK_BYTES: u32 = 1024 * 1024;
pub const DEFAULT_MAX_FRAMES_PER_PUSH: usize = 64;
pub const DEFAULT_MAX_IN_FLIGHT_REQUESTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLimits {
    max_control_body_bytes: u32,
    max_bulk_chunk_bytes: u32,
    max_frames_per_push: usize,
    max_in_flight_requests: usize,
}

impl FrameLimits {
    pub const fn new(
        max_control_body_bytes: u32,
        max_bulk_chunk_bytes: u32,
        max_frames_per_push: usize,
        max_in_flight_requests: usize,
    ) -> Result<Self, IpcError> {
        if max_control_body_bytes == 0 {
            return Err(IpcError::InvalidLimit("max control body bytes"));
        }
        if max_bulk_chunk_bytes == 0 {
            return Err(IpcError::InvalidLimit("max bulk chunk bytes"));
        }
        if max_frames_per_push == 0 {
            return Err(IpcError::InvalidLimit("max frames per push"));
        }
        if max_in_flight_requests == 0 {
            return Err(IpcError::InvalidLimit("max in-flight requests"));
        }
        Ok(Self {
            max_control_body_bytes,
            max_bulk_chunk_bytes,
            max_frames_per_push,
            max_in_flight_requests,
        })
    }

    #[must_use]
    pub const fn max_control_body_bytes(self) -> u32 {
        self.max_control_body_bytes
    }

    #[must_use]
    pub const fn max_bulk_chunk_bytes(self) -> u32 {
        self.max_bulk_chunk_bytes
    }

    #[must_use]
    pub const fn max_frames_per_push(self) -> usize {
        self.max_frames_per_push
    }

    #[must_use]
    pub const fn max_in_flight_requests(self) -> usize {
        self.max_in_flight_requests
    }
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_control_body_bytes: DEFAULT_MAX_CONTROL_BODY_BYTES,
            max_bulk_chunk_bytes: DEFAULT_MAX_BULK_CHUNK_BYTES,
            max_frames_per_push: DEFAULT_MAX_FRAMES_PER_PUSH,
            max_in_flight_requests: DEFAULT_MAX_IN_FLIGHT_REQUESTS,
        }
    }
}
