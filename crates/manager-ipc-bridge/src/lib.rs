#![cfg(windows)]
#![deny(unsafe_code)]

mod abi;
mod client;
#[allow(unsafe_code)]
mod ffi;

pub use abi::{
    PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED, PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK,
    PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK, PASTRAL_MANAGER_IPC_ABI_VERSION,
    PASTRAL_MANAGER_IPC_RESULT_BYTES, PastralManagerHealthResult, PastralManagerHealthStatus,
};
pub use client::{ManagerHealthSnapshot, ManagerHealthStatus, query_health};
pub use ffi::{
    pastral_manager_ipc_abi_version, pastral_manager_ipc_health_w, pastral_manager_ipc_result_size,
};
