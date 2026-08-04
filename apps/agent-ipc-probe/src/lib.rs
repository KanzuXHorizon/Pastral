#![cfg(windows)]
#![forbid(unsafe_code)]

mod child;
mod cli;
mod error;

pub use child::run_baseline_child;
pub use cli::{AdmissionMode, parse_arguments};
pub use error::AdmissionError;
