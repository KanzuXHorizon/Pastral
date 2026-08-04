use crate::{PipeName, PipeSecurity, TransportError, sys};

pub struct PipeServer {
    _native: sys::OwnedPipeHandle,
}

pub fn create_first_pipe_server(
    name: &PipeName,
    security: &PipeSecurity,
) -> Result<PipeServer, TransportError> {
    Ok(PipeServer {
        _native: sys::create_first_named_pipe(name.as_wide_nul(), security.native())?,
    })
}
