use std::{env, io, process::ExitCode, time::Duration};

use pastral_agent::{
    AgentIpcCommand, HealthServerConfig, ipc_usage, parse_ipc_arguments, serve_health,
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

    match command {
        AgentIpcCommand::ServeHealth {
            data_root,
            max_connections,
        } => {
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
            match serve_health(config, &mut output) {
                Ok(_) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
