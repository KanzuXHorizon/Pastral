use std::{io::Write, num::NonZeroUsize, path::Path, time::Duration};

use pastral_agent::{AgentIpcError, HealthServerConfig, serve_health};

use crate::AdmissionError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);

pub fn run_server_child<W: Write>(data_root: &Path, mut output: W) -> Result<(), AdmissionError> {
    let config = HealthServerConfig::new(
        data_root.to_path_buf(),
        NonZeroUsize::MIN,
        CONNECT_TIMEOUT,
        OPERATION_TIMEOUT,
    )
    .map_err(map_error)?
    .without_summary();
    serve_health(config, &mut output)
        .map(|_| ())
        .map_err(map_error)
}

fn map_error(error: AgentIpcError) -> AdmissionError {
    match error {
        AgentIpcError::Io { operation, kind } => AdmissionError::Io { operation, kind },
        AgentIpcError::InvalidConfiguration => AdmissionError::InvalidArguments,
        AgentIpcError::AgentHealth => AdmissionError::AgentHealth,
        AgentIpcError::Material => AdmissionError::Material,
        AgentIpcError::Transport => AdmissionError::Transport,
        AgentIpcError::Authentication => AdmissionError::Authentication,
        AgentIpcError::Protocol => AdmissionError::Protocol,
    }
}
