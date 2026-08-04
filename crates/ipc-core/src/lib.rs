#![forbid(unsafe_code)]

mod decoder;
mod error;
mod frame;
mod limits;

pub use decoder::{Frame, FrameDecoder};
pub use error::IpcError;
pub use frame::{CorrelationId, FRAME_HEADER_BYTES, FRAMING_MAJOR, FrameHeader, FrameKind};
pub use limits::{
    DEFAULT_MAX_BULK_CHUNK_BYTES, DEFAULT_MAX_CONTROL_BODY_BYTES, DEFAULT_MAX_FRAMES_PER_PUSH,
    DEFAULT_MAX_IN_FLIGHT_REQUESTS, FrameLimits,
};
