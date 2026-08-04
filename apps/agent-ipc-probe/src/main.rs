use std::{env, io, process::ExitCode};

use pastral_agent_ipc_probe::{
    AdmissionError, AdmissionMode, parse_arguments, run_baseline_child, run_parent,
    run_server_child,
};

fn main() -> ExitCode {
    let mode = match parse_arguments(env::args_os().skip(1)) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("agent-ipc-admission={error}");
            return ExitCode::from(2);
        }
    };

    let result = match mode {
        AdmissionMode::Parent => {
            let stdout = io::stdout();
            run_parent(stdout.lock())
        }
        AdmissionMode::BaselineChild { data_root } => {
            let stdin = io::stdin();
            let stdout = io::stdout();
            run_baseline_child(&data_root, stdin.lock(), stdout.lock())
        }
        AdmissionMode::ServerChild { data_root } => {
            let stdout = io::stdout();
            run_server_child(&data_root, stdout.lock())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agent-ipc-admission={}", error_class(&error));
            ExitCode::from(1)
        }
    }
}

fn error_class(error: &AdmissionError) -> &'static str {
    match error {
        AdmissionError::InvalidArguments => "invalid arguments",
        AdmissionError::AgentHealth => "agent Health failed",
        AdmissionError::InvalidChildInput => "invalid child input",
        AdmissionError::Environment => "environment failed",
        AdmissionError::Material => "material failed",
        AdmissionError::Process => "process failed",
        AdmissionError::Readiness => "readiness failed",
        AdmissionError::Transport => "transport failed",
        AdmissionError::Authentication => "authentication failed",
        AdmissionError::Protocol => "protocol failed",
        AdmissionError::ChildFailure => "child failed",
        AdmissionError::MissingReleaseArtifact => "release artifact missing",
        AdmissionError::InvalidMetric => "metric invalid",
        AdmissionError::FootprintCeiling => "footprint ceiling failed",
        AdmissionError::Cleanup => "cleanup failed",
        AdmissionError::Timeout => "timeout",
        AdmissionError::Io { .. } => "I/O failed",
    }
}
