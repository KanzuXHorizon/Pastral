use std::{env, io, process::ExitCode};

use pastral_agent::{parse_arguments, run_command, usage};

fn main() -> ExitCode {
    let command = match parse_arguments(env::args_os().skip(1)) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    };

    let stdout = io::stdout();
    let mut output = stdout.lock();
    match run_command(command, &mut output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
