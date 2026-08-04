use std::time::Duration;

use crate::{ManagerHealthSnapshot, ManagerHealthStatus};

pub const PASTRAL_MANAGER_IPC_ABI_VERSION: u32 = 1;
pub const PASTRAL_MANAGER_IPC_RESULT_BYTES: u32 = 64;
pub const PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED: u32 = 1 << 0;
pub const PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK: u32 = 1 << 1;
pub const PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK: u32 = 1 << 2;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PastralManagerHealthStatus {
    Connected = 0,
    Disconnected = 1,
    Timeout = 2,
    ProtocolMismatch = 3,
    AuthenticationFailed = 4,
    Unhealthy = 5,
    InvalidArgument = 6,
    InternalError = 7,
    AbiMismatch = 8,
}

impl PastralManagerHealthStatus {
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

impl From<ManagerHealthStatus> for PastralManagerHealthStatus {
    fn from(value: ManagerHealthStatus) -> Self {
        match value {
            ManagerHealthStatus::Connected => Self::Connected,
            ManagerHealthStatus::Disconnected => Self::Disconnected,
            ManagerHealthStatus::Timeout => Self::Timeout,
            ManagerHealthStatus::ProtocolMismatch => Self::ProtocolMismatch,
            ManagerHealthStatus::AuthenticationFailed => Self::AuthenticationFailed,
            ManagerHealthStatus::Unhealthy => Self::Unhealthy,
            ManagerHealthStatus::InternalError => Self::InternalError,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PastralManagerHealthResult {
    pub abi_version: u32,
    pub struct_size: u32,
    pub status: u32,
    pub storage_schema_version: u32,
    pub integrity_flags: u32,
    pub server_process_id: u32,
    pub session_id: u32,
    pub reserved0: u32,
    pub connect_us: u64,
    pub handshake_us: u64,
    pub health_us: u64,
    pub reserved1: u64,
}

impl PastralManagerHealthResult {
    #[must_use]
    pub const fn failed(status: PastralManagerHealthStatus) -> Self {
        Self {
            abi_version: PASTRAL_MANAGER_IPC_ABI_VERSION,
            struct_size: PASTRAL_MANAGER_IPC_RESULT_BYTES,
            status: status as u32,
            storage_schema_version: 0,
            integrity_flags: 0,
            server_process_id: 0,
            session_id: 0,
            reserved0: 0,
            connect_us: 0,
            handshake_us: 0,
            health_us: 0,
            reserved1: 0,
        }
    }

    #[must_use]
    pub fn from_snapshot(snapshot: ManagerHealthSnapshot) -> Self {
        let status = PastralManagerHealthStatus::from(snapshot.status());
        let mut flags = 0u32;
        if snapshot.capture_enabled() {
            flags |= PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED;
        }
        if snapshot.privacy_policy_ok() {
            flags |= PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK;
        }
        if snapshot.storage_integrity_ok() {
            flags |= PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK;
        }
        Self {
            abi_version: PASTRAL_MANAGER_IPC_ABI_VERSION,
            struct_size: PASTRAL_MANAGER_IPC_RESULT_BYTES,
            status: status as u32,
            storage_schema_version: snapshot.storage_schema_version(),
            integrity_flags: flags,
            server_process_id: snapshot.server_process_id(),
            session_id: snapshot.session_id(),
            reserved0: 0,
            connect_us: duration_micros(snapshot.connect_elapsed()),
            handshake_us: duration_micros(snapshot.handshake_elapsed()),
            health_us: duration_micros(snapshot.health_elapsed()),
            reserved1: 0,
        }
    }
}

fn duration_micros(value: Duration) -> u64 {
    u64::try_from(value.as_micros()).unwrap_or(u64::MAX)
}
