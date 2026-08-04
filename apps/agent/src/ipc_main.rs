use std::{env, io, process::ExitCode, time::Duration};

use pastral_agent::{
    AgentIpcCommand, HealthServerConfig, ipc_usage, parse_ipc_arguments, serve_health, serve_read,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);

fn main() -> ExitCode {
    let command = match parse_ipc_arguments(env::args_os().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{}", ipc_usage());
            return ExitCode::from(2);
        }
    };

    let (data_root, max_connections, read_only) = match command {
        AgentIpcCommand::ServeHealth {
            data_root,
            max_connections,
        } => (data_root, max_connections, false),
        AgentIpcCommand::ServeRead {
            data_root,
            max_connections,
        } => (data_root, max_connections, true),
    };
    let config = match HealthServerConfig::new(
        data_root,
        max_connections,
        CONNECT_TIMEOUT,
        OPERATION_TIMEOUT,
    ) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let result = if read_only {
        serve_read(config, &mut output)
    } else {
        serve_health(config, &mut output)
    };
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
