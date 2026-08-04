use pastral_ipc_core::{CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits};

use crate::AdmissionError;

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
