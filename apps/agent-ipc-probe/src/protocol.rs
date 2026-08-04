use pastral_agent::AgentHealthSnapshot;
use pastral_ipc_core::{
    CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthResponseDto, ResponseDto,
};
use pastral_ipc_schema::encode_response;

use crate::AdmissionError;

pub(crate) fn health_response(
    snapshot: &AgentHealthSnapshot,
) -> Result<ResponseDto, AdmissionError> {
    Ok(ResponseDto::Health(
        HealthResponseDto::new(
            snapshot.storage_schema_version(),
            snapshot.capture_enabled(),
            snapshot.privacy_policy_ok(),
            snapshot.storage_integrity_ok(),
        )
        .map_err(|_| AdmissionError::Protocol)?,
    ))
}

pub(crate) fn control_frame(
    body: Vec<u8>,
    correlation: CorrelationId,
) -> Result<Frame, AdmissionError> {
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).map_err(|_| AdmissionError::Protocol)?,
        0,
        correlation,
        FrameLimits::default(),
    )
    .map_err(|_| AdmissionError::Protocol)?;
    Frame::new(header, body).map_err(|_| AdmissionError::Protocol)
}

pub(crate) fn health_response_frame(
    snapshot: &AgentHealthSnapshot,
    correlation: CorrelationId,
) -> Result<Frame, AdmissionError> {
    let body =
        encode_response(&health_response(snapshot)?).map_err(|_| AdmissionError::Protocol)?;
    control_frame(body, correlation)
}
