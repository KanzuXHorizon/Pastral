#![cfg(windows)]
#![forbid(unsafe_code)]

mod cli;
mod clock;
mod config;
mod error;
mod health;
#[cfg(feature = "ipc-health")]
mod ipc_cli;
#[cfg(feature = "ipc-health")]
mod ipc_health;
mod platform;
mod privacy_config;
mod runtime;
mod storage_sink;

pub use cli::{AgentCommand, CliError, parse_arguments, usage};
pub use clock::{SystemClock, ThreadSleeper};
pub use config::AgentIdentity;
pub use error::AgentRuntimeError;
pub use health::{AgentHealthSnapshot, load_health_snapshot};
#[cfg(feature = "ipc-health")]
pub use ipc_cli::{AgentIpcCliError, AgentIpcCommand, ipc_usage, parse_ipc_arguments};
#[cfg(feature = "ipc-health")]
pub use ipc_health::{
    AgentIpcError, HealthServerConfig, HealthServerSummary, ResidentReadServerConfig, serve_health,
    serve_read, serve_read_until_stopped,
};
pub use platform::WindowsClipboardSource;
pub use privacy_config::PrivacyPolicyConfig;
pub use runtime::{resolve_resident_data_root, resolve_resident_data_root_from, run_command};
pub use storage_sink::{DiagnosticStoragePolicy, StorageCaptureSink, diagnostic_storage_limits};
