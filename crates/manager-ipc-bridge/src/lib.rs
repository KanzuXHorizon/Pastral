#![cfg(windows)]
#![deny(unsafe_code)]

mod client;

pub use client::{ManagerHealthSnapshot, ManagerHealthStatus, query_health};
