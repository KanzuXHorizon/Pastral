#![cfg(windows)]
#![forbid(unsafe_code)]

mod cli;
mod clock;
mod config;
mod error;
mod platform;
mod privacy_config;
mod runtime;
mod storage_sink;

pub use cli::{AgentCommand, CliError, parse_arguments, usage};
pub use clock::{SystemClock, ThreadSleeper};
pub use config::AgentIdentity;
pub use error::AgentRuntimeError;
pub use platform::WindowsClipboardSource;
pub use privacy_config::PrivacyPolicyConfig;
pub use runtime::run_command;
pub use storage_sink::{DiagnosticStoragePolicy, StorageCaptureSink, diagnostic_storage_limits};
