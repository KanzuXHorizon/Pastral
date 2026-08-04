#![cfg(windows)]
#![deny(unsafe_code)]

mod abi;
mod client;
#[allow(unsafe_code)]
mod ffi;

pub use abi::{
    PASTRAL_MANAGER_CLIP_ITEM_BYTES, PASTRAL_MANAGER_CLIP_KIND_TEXT,
    PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE, PASTRAL_MANAGER_CLIP_PINNED,
    PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED, PASTRAL_MANAGER_CLIP_UNAVAILABLE,
    PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED, PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK,
    PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK, PASTRAL_MANAGER_IPC_ABI_VERSION,
    PASTRAL_MANAGER_IPC_RESULT_BYTES, PASTRAL_MANAGER_READ_ABI_VERSION,
    PASTRAL_MANAGER_READ_RESULT_BYTES, PastralManagerClipItem, PastralManagerHealthResult,
    PastralManagerHealthStatus, PastralManagerReadResult,
};
pub use client::{ManagerHealthSnapshot, ManagerHealthStatus, query_health};
pub use ffi::{
    pastral_manager_ipc_abi_version, pastral_manager_ipc_clip_item_size,
    pastral_manager_ipc_health_w, pastral_manager_ipc_read_abi_version,
    pastral_manager_ipc_read_result_size, pastral_manager_ipc_result_size,
};
