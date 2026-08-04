use std::{thread, time::Instant};

use crate::{
    PipeName, PipeSecurity, TokenIdentity, TransportError, ValidatedPeer, current_token_identity,
    overlapped::remaining_millis, process_token_identity, sys, validate_peer,
};

pub struct PipeServer {
    native: sys::OwnedPipeHandle,
}

impl PipeServer {
    pub fn connect(&mut self, deadline: Instant) -> Result<(), TransportError> {
        sys::connect_named_pipe(
            &self.native,
            remaining_millis(deadline, "connect named pipe")?,
        )
    }

    pub fn peer_identity(&self) -> Result<ValidatedPeer, TransportError> {
        let current = current_token_identity()?;
        let (process_id, session_id) = sys::named_pipe_client_endpoint(&self.native)?;
        validate_kernel_peer(&current, process_id, session_id)
    }

    pub(crate) fn into_native(self) -> sys::OwnedPipeHandle {
        self.native
    }
}

pub struct PipeClient {
    native: sys::OwnedPipeHandle,
}

impl PipeClient {
    pub fn peer_identity(&self) -> Result<ValidatedPeer, TransportError> {
        let current = current_token_identity()?;
        let (process_id, session_id) = sys::named_pipe_server_endpoint(&self.native)?;
        validate_kernel_peer(&current, process_id, session_id)
    }

    pub(crate) fn into_native(self) -> sys::OwnedPipeHandle {
        self.native
    }
}

pub fn create_first_pipe_server(
    name: &PipeName,
    security: &PipeSecurity,
) -> Result<PipeServer, TransportError> {
    Ok(PipeServer {
        native: sys::create_first_named_pipe(name.as_wide_nul(), security.native())?,
    })
}

pub fn open_pipe_client(name: &PipeName, deadline: Instant) -> Result<PipeClient, TransportError> {
    loop {
        let remaining = remaining_millis(deadline, "open named-pipe client")?;
        if !sys::wait_named_pipe(name.as_wide_nul(), remaining)? {
            if Instant::now() >= deadline {
                return Err(TransportError::Timeout("open named-pipe client"));
            }
            thread::yield_now();
            continue;
        }
        if let Some(native) = sys::open_named_pipe_client(name.as_wide_nul())? {
            return Ok(PipeClient { native });
        }
        if Instant::now() >= deadline {
            return Err(TransportError::Timeout("open named-pipe client"));
        }
        thread::yield_now();
    }
}

fn validate_kernel_peer(
    current: &TokenIdentity,
    process_id: u32,
    session_id: u32,
) -> Result<ValidatedPeer, TransportError> {
    let observed = process_token_identity(process_id)?;
    validate_peer(current, process_id, session_id, &observed).map_err(|_| {
        TransportError::InvalidTokenIdentity("named-pipe peer token evidence mismatch")
    })
}
