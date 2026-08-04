use core::cmp;

use crate::{FRAME_HEADER_BYTES, FrameHeader, FrameLimits, IpcError};

#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    header: FrameHeader,
    body: Vec<u8>,
}

impl Frame {
    pub fn new(header: FrameHeader, body: Vec<u8>) -> Result<Self, IpcError> {
        let actual = body.len();
        let declared = header.body_length();
        if usize::try_from(declared).map_err(|_| IpcError::IntegerOverflow)? != actual {
            return Err(IpcError::FrameBodyLengthMismatch { declared, actual });
        }
        Ok(Self { header, body })
    }

    #[must_use]
    pub const fn header(&self) -> FrameHeader {
        self.header
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

impl core::fmt::Debug for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Frame")
            .field("header", &self.header)
            .field("body_length", &self.body.len())
            .finish()
    }
}

enum DecoderState {
    Header {
        bytes: [u8; FRAME_HEADER_BYTES],
        filled: usize,
    },
    Body {
        header: FrameHeader,
        bytes: Box<[u8]>,
        filled: usize,
    },
    Poisoned,
}

impl DecoderState {
    const fn empty_header() -> Self {
        Self::Header {
            bytes: [0; FRAME_HEADER_BYTES],
            filled: 0,
        }
    }
}

pub struct FrameDecoder {
    limits: FrameLimits,
    state: DecoderState,
    allocated_body_capacity: usize,
}

impl FrameDecoder {
    #[must_use]
    pub const fn new(limits: FrameLimits) -> Self {
        Self {
            limits,
            state: DecoderState::empty_header(),
            allocated_body_capacity: 0,
        }
    }

    pub fn push(&mut self, input: &[u8]) -> Result<Vec<Frame>, IpcError> {
        if matches!(self.state, DecoderState::Poisoned) {
            return Err(IpcError::DecoderPoisoned);
        }

        let mut remaining = input;
        let mut frames = Vec::new();
        while !remaining.is_empty() {
            let state = core::mem::replace(&mut self.state, DecoderState::Poisoned);
            match state {
                DecoderState::Header {
                    mut bytes,
                    mut filled,
                } => {
                    let copy_count = cmp::min(FRAME_HEADER_BYTES - filled, remaining.len());
                    bytes[filled..filled + copy_count].copy_from_slice(&remaining[..copy_count]);
                    filled += copy_count;
                    remaining = &remaining[copy_count..];

                    if filled < FRAME_HEADER_BYTES {
                        self.state = DecoderState::Header { bytes, filled };
                        continue;
                    }

                    let header = match FrameHeader::decode(&bytes, self.limits) {
                        Ok(value) => value,
                        Err(error) => return self.poison(error),
                    };
                    let body_length = usize::try_from(header.body_length())
                        .map_err(|_| IpcError::IntegerOverflow)?;
                    if body_length == 0 {
                        if frames.len() >= self.limits.max_frames_per_push() {
                            return self.poison(IpcError::TooManyFrames);
                        }
                        frames.push(Frame {
                            header,
                            body: Vec::new(),
                        });
                        self.state = DecoderState::empty_header();
                    } else {
                        let bytes = vec![0u8; body_length].into_boxed_slice();
                        self.allocated_body_capacity =
                            self.allocated_body_capacity.max(bytes.len());
                        self.state = DecoderState::Body {
                            header,
                            bytes,
                            filled: 0,
                        };
                    }
                }
                DecoderState::Body {
                    header,
                    mut bytes,
                    mut filled,
                } => {
                    let copy_count = cmp::min(bytes.len() - filled, remaining.len());
                    bytes[filled..filled + copy_count].copy_from_slice(&remaining[..copy_count]);
                    filled += copy_count;
                    remaining = &remaining[copy_count..];

                    if filled < bytes.len() {
                        self.state = DecoderState::Body {
                            header,
                            bytes,
                            filled,
                        };
                        continue;
                    }
                    if frames.len() >= self.limits.max_frames_per_push() {
                        return self.poison(IpcError::TooManyFrames);
                    }
                    frames.push(Frame {
                        header,
                        body: bytes.into_vec(),
                    });
                    self.state = DecoderState::empty_header();
                }
                DecoderState::Poisoned => return Err(IpcError::DecoderPoisoned),
            }
        }
        Ok(frames)
    }

    pub fn finish(self) -> Result<(), IpcError> {
        match self.state {
            DecoderState::Header { filled: 0, .. } => Ok(()),
            DecoderState::Poisoned => Err(IpcError::DecoderPoisoned),
            DecoderState::Header { .. } | DecoderState::Body { .. } => {
                Err(IpcError::TruncatedFrame)
            }
        }
    }

    #[must_use]
    pub const fn allocated_body_capacity(&self) -> usize {
        self.allocated_body_capacity
    }

    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        matches!(self.state, DecoderState::Poisoned)
    }

    fn poison<T>(&mut self, error: IpcError) -> Result<T, IpcError> {
        self.state = DecoderState::Poisoned;
        Err(error)
    }
}
