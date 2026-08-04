#![forbid(unsafe_code)]

mod connection;
mod decoder;
mod dto;
mod error;
mod frame;
mod limits;

pub use connection::{AcceptedFrame, BulkProgress, ConnectionPhase, ServerConnection};
pub use decoder::{Frame, FrameDecoder};
pub use dto::{
    BulkEndDto, Capability, ClientHelloDto, ClipPreviewDto, ClipPreviewKind, HealthRequestDto,
    HealthResponseDto, HistoryPageRequestDto, HistoryPageResponseDto, MAX_ERROR_DETAIL_BYTES,
    MAX_PAGE_LIMIT, MAX_PREVIEW_BYTES, MAX_PREVIEWS, MAX_QUERY_BYTES, MAX_QUERY_TERMS,
    MAX_SOURCE_LABEL_BYTES, NONCE_BYTES, ProtocolErrorCode, ProtocolErrorDto, RequestDto,
    ResponseDto, SearchRequestDto, SearchResponseDto, ServerHelloDto,
};
pub use error::IpcError;
pub use frame::{CorrelationId, FRAME_HEADER_BYTES, FRAMING_MAJOR, FrameHeader, FrameKind};
pub use limits::{
    DEFAULT_MAX_BULK_CHUNK_BYTES, DEFAULT_MAX_CONTROL_BODY_BYTES, DEFAULT_MAX_FRAMES_PER_PUSH,
    DEFAULT_MAX_IN_FLIGHT_REQUESTS, FrameLimits,
};
