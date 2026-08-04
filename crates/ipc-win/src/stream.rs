use std::{collections::VecDeque, time::Instant};

use pastral_ipc_core::{Frame, FrameDecoder, FrameLimits};

use crate::{PipeClient, PipeServer, TransportError, overlapped::remaining_millis, sys};

const PIPE_READ_BUFFER_BYTES: usize = 64 * 1024;

pub struct PipeFrameStream {
    native: sys::OwnedPipeHandle,
    decoder: FrameDecoder,
    pending: VecDeque<Frame>,
}

impl PipeFrameStream {
    #[must_use]
    pub fn from_server(server: PipeServer, limits: FrameLimits) -> Self {
        Self {
            native: server.into_native(),
            decoder: FrameDecoder::new(limits),
            pending: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn from_client(client: PipeClient, limits: FrameLimits) -> Self {
        Self {
            native: client.into_native(),
            decoder: FrameDecoder::new(limits),
            pending: VecDeque::new(),
        }
    }

    pub fn write_frame(&mut self, frame: &Frame, deadline: Instant) -> Result<(), TransportError> {
        let header = frame.header().encode();
        self.write_all(&header, deadline)?;
        self.write_all(frame.body(), deadline)
    }

    pub fn read_frame(&mut self, deadline: Instant) -> Result<Frame, TransportError> {
        if let Some(frame) = self.pending.pop_front() {
            return Ok(frame);
        }

        let mut buffer = [0u8; PIPE_READ_BUFFER_BYTES];
        loop {
            let count = sys::read_pipe(
                &self.native,
                &mut buffer,
                remaining_millis(deadline, "read IPC frame")?,
            )?;
            let frames = self
                .decoder
                .push(&buffer[..count])
                .map_err(|_| TransportError::Protocol("frame decoder rejected byte stream"))?;
            self.pending.extend(frames);
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
        }
    }

    fn write_all(&mut self, mut bytes: &[u8], deadline: Instant) -> Result<(), TransportError> {
        while !bytes.is_empty() {
            let written = sys::write_pipe(
                &self.native,
                bytes,
                remaining_millis(deadline, "write IPC frame")?,
            )?;
            if written == 0 || written > bytes.len() {
                return Err(TransportError::Protocol(
                    "pipe write returned invalid byte count",
                ));
            }
            bytes = &bytes[written..];
        }
        Ok(())
    }
}
