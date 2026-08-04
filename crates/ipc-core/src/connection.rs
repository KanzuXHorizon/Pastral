use std::collections::BTreeSet;

use crate::{CorrelationId, Frame, FrameKind, FrameLimits, IpcError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionPhase {
    AwaitClientHello,
    Ready,
    BulkReceiving,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkProgress {
    transfer: CorrelationId,
    next_sequence: u32,
    accepted_chunks: u32,
    accepted_bytes: u64,
    max_chunks: u32,
    max_bytes: u64,
    end_received: bool,
}

impl BulkProgress {
    #[must_use]
    pub const fn transfer(self) -> CorrelationId {
        self.transfer
    }

    #[must_use]
    pub const fn next_sequence(self) -> u32 {
        self.next_sequence
    }

    #[must_use]
    pub const fn accepted_chunks(self) -> u32 {
        self.accepted_chunks
    }

    #[must_use]
    pub const fn accepted_bytes(self) -> u64 {
        self.accepted_bytes
    }

    #[must_use]
    pub const fn max_chunks(self) -> u32 {
        self.max_chunks
    }

    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }

    #[must_use]
    pub const fn end_received(self) -> bool {
        self.end_received
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptedFrame<'a> {
    ClientHello {
        correlation: CorrelationId,
        body: &'a [u8],
    },
    Control {
        correlation: CorrelationId,
        body: &'a [u8],
    },
    BulkChunk {
        transfer: CorrelationId,
        sequence: u32,
        bytes: &'a [u8],
    },
    BulkEnd {
        transfer: CorrelationId,
        accepted_chunks: u32,
        accepted_bytes: u64,
        body: &'a [u8],
    },
}

#[derive(Debug, Clone, Copy)]
struct BulkState {
    progress: BulkProgress,
}

pub struct ServerConnection {
    limits: FrameLimits,
    phase: ConnectionPhase,
    in_flight: BTreeSet<CorrelationId>,
    bulk: Option<BulkState>,
}

impl ServerConnection {
    #[must_use]
    pub fn new(limits: FrameLimits) -> Self {
        Self {
            limits,
            phase: ConnectionPhase::AwaitClientHello,
            in_flight: BTreeSet::new(),
            bulk: None,
        }
    }

    pub fn accept<'a>(&mut self, frame: &'a Frame) -> Result<AcceptedFrame<'a>, IpcError> {
        match self.phase {
            ConnectionPhase::AwaitClientHello => self.accept_client_hello(frame),
            ConnectionPhase::Ready => self.accept_ready(frame),
            ConnectionPhase::BulkReceiving => self.accept_bulk(frame),
            ConnectionPhase::Closed => Err(IpcError::InvalidConnectionState),
        }
    }

    pub fn complete_request(&mut self, correlation: CorrelationId) -> Result<(), IpcError> {
        if self.phase == ConnectionPhase::Closed {
            return Err(IpcError::InvalidConnectionState);
        }
        if self.in_flight.remove(&correlation) {
            Ok(())
        } else {
            Err(IpcError::UnknownCorrelation)
        }
    }

    pub fn cancel_request(&mut self, correlation: CorrelationId) -> Result<(), IpcError> {
        self.complete_request(correlation)
    }

    pub fn authorize_bulk(
        &mut self,
        transfer: CorrelationId,
        max_bytes: u64,
        max_chunks: u32,
    ) -> Result<(), IpcError> {
        if self.phase != ConnectionPhase::Ready || self.bulk.is_some() || transfer.is_zero() {
            return Err(IpcError::InvalidConnectionState);
        }
        if max_bytes == 0 {
            return Err(IpcError::InvalidLimit("bulk maximum bytes"));
        }
        if max_chunks == 0 {
            return Err(IpcError::InvalidLimit("bulk maximum chunks"));
        }
        self.bulk = Some(BulkState {
            progress: BulkProgress {
                transfer,
                next_sequence: 0,
                accepted_chunks: 0,
                accepted_bytes: 0,
                max_chunks,
                max_bytes,
                end_received: false,
            },
        });
        self.phase = ConnectionPhase::BulkReceiving;
        Ok(())
    }

    pub fn complete_bulk(
        &mut self,
        declared_bytes: u64,
        declared_chunks: u32,
    ) -> Result<(), IpcError> {
        if self.phase != ConnectionPhase::BulkReceiving {
            return Err(IpcError::InvalidConnectionState);
        }
        let Some(state) = self.bulk else {
            return self.fail(IpcError::BulkNotAuthorized);
        };
        let progress = state.progress;
        if !progress.end_received
            || progress.accepted_bytes != declared_bytes
            || progress.accepted_chunks != declared_chunks
        {
            return self.fail(IpcError::BulkEndMismatch);
        }
        self.bulk = None;
        self.phase = ConnectionPhase::Ready;
        Ok(())
    }

    pub fn cancel_bulk(&mut self) -> Result<(), IpcError> {
        if self.phase != ConnectionPhase::BulkReceiving || self.bulk.is_none() {
            return Err(IpcError::BulkNotAuthorized);
        }
        self.bulk = None;
        self.phase = ConnectionPhase::Ready;
        Ok(())
    }

    #[must_use]
    pub const fn phase(&self) -> ConnectionPhase {
        self.phase
    }

    #[must_use]
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    #[must_use]
    pub fn bulk_progress(&self) -> Option<BulkProgress> {
        self.bulk.map(|state| state.progress)
    }

    fn accept_client_hello<'a>(&mut self, frame: &'a Frame) -> Result<AcceptedFrame<'a>, IpcError> {
        let header = frame.header();
        if header.kind() != FrameKind::HelloProto {
            return self.fail(IpcError::InvalidConnectionState);
        }
        if header.correlation().is_zero() {
            return self.fail(IpcError::InvalidCorrelation);
        }
        self.phase = ConnectionPhase::Ready;
        Ok(AcceptedFrame::ClientHello {
            correlation: header.correlation(),
            body: frame.body(),
        })
    }

    fn accept_ready<'a>(&mut self, frame: &'a Frame) -> Result<AcceptedFrame<'a>, IpcError> {
        let header = frame.header();
        match header.kind() {
            FrameKind::ControlProto => {
                let correlation = header.correlation();
                if self.in_flight.contains(&correlation) {
                    return self.fail(IpcError::DuplicateCorrelation);
                }
                if self.in_flight.len() >= self.limits.max_in_flight_requests() {
                    return self.fail(IpcError::InFlightLimitExceeded);
                }
                self.in_flight.insert(correlation);
                Ok(AcceptedFrame::Control {
                    correlation,
                    body: frame.body(),
                })
            }
            FrameKind::BulkChunk | FrameKind::BulkEndProto => {
                self.fail(IpcError::BulkNotAuthorized)
            }
            FrameKind::HelloProto | FrameKind::ProtocolErrorProto => {
                self.fail(IpcError::InvalidConnectionState)
            }
        }
    }

    fn accept_bulk<'a>(&mut self, frame: &'a Frame) -> Result<AcceptedFrame<'a>, IpcError> {
        let Some(mut state) = self.bulk else {
            return self.fail(IpcError::BulkNotAuthorized);
        };
        if state.progress.end_received {
            return self.fail(IpcError::InvalidConnectionState);
        }
        let header = frame.header();
        if header.correlation() != state.progress.transfer {
            return self.fail(IpcError::InvalidCorrelation);
        }

        match header.kind() {
            FrameKind::BulkChunk => {
                if header.sequence() != state.progress.next_sequence {
                    return self.fail(IpcError::BulkSequenceMismatch {
                        expected: state.progress.next_sequence,
                        actual: header.sequence(),
                    });
                }
                if state.progress.accepted_chunks >= state.progress.max_chunks {
                    return self.fail(IpcError::BulkChunkLimitExceeded);
                }
                let body_length =
                    u64::try_from(frame.body().len()).map_err(|_| IpcError::IntegerOverflow)?;
                let accepted_bytes = state
                    .progress
                    .accepted_bytes
                    .checked_add(body_length)
                    .ok_or(IpcError::IntegerOverflow)?;
                if accepted_bytes > state.progress.max_bytes {
                    return self.fail(IpcError::BulkLengthExceeded);
                }
                let next_sequence = state
                    .progress
                    .next_sequence
                    .checked_add(1)
                    .ok_or(IpcError::IntegerOverflow)?;
                let accepted_chunks = state
                    .progress
                    .accepted_chunks
                    .checked_add(1)
                    .ok_or(IpcError::IntegerOverflow)?;
                state.progress.accepted_bytes = accepted_bytes;
                state.progress.accepted_chunks = accepted_chunks;
                state.progress.next_sequence = next_sequence;
                self.bulk = Some(state);
                Ok(AcceptedFrame::BulkChunk {
                    transfer: header.correlation(),
                    sequence: header.sequence(),
                    bytes: frame.body(),
                })
            }
            FrameKind::BulkEndProto => {
                if header.sequence() != state.progress.accepted_chunks {
                    return self.fail(IpcError::BulkSequenceMismatch {
                        expected: state.progress.accepted_chunks,
                        actual: header.sequence(),
                    });
                }
                state.progress.end_received = true;
                self.bulk = Some(state);
                Ok(AcceptedFrame::BulkEnd {
                    transfer: header.correlation(),
                    accepted_chunks: state.progress.accepted_chunks,
                    accepted_bytes: state.progress.accepted_bytes,
                    body: frame.body(),
                })
            }
            FrameKind::HelloProto | FrameKind::ControlProto | FrameKind::ProtocolErrorProto => {
                self.fail(IpcError::InvalidConnectionState)
            }
        }
    }

    fn fail<T>(&mut self, error: IpcError) -> Result<T, IpcError> {
        self.phase = ConnectionPhase::Closed;
        self.in_flight.clear();
        self.bulk = None;
        Err(error)
    }
}
