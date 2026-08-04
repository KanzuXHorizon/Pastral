#![cfg(windows)]
#![forbid(unsafe_code)]

mod child;
mod cli;
mod error;
mod metrics;
mod parent;
mod protocol;
mod server;

pub use child::run_baseline_child;
pub use cli::{AdmissionMode, parse_arguments};
pub use error::AdmissionError;
pub use metrics::{FootprintMetrics, calculate_footprint, enforce_footprint, evaluate_footprint};
pub use parent::run_parent;
pub use server::run_server_child;
