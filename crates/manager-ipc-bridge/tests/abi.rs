#![cfg(windows)]

use std::{fs, mem};

use pastral_manager_ipc_bridge::{
    PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES, PastralManagerHealthResult,
    PastralManagerHealthStatus, pastral_manager_ipc_abi_version, pastral_manager_ipc_result_size,
};

#[test]
fn result_layout_and_status_values_are_exact() {
    assert_eq!(PASTRAL_MANAGER_IPC_ABI_VERSION, 1);
    assert_eq!(PASTRAL_MANAGER_IPC_RESULT_BYTES, 64);
    assert_eq!(mem::size_of::<PastralManagerHealthResult>(), 64);
    assert_eq!(mem::align_of::<PastralManagerHealthResult>(), 8);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, abi_version), 0);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, struct_size), 4);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, status), 8);
    assert_eq!(
        mem::offset_of!(PastralManagerHealthResult, storage_schema_version),
        12
    );
    assert_eq!(
        mem::offset_of!(PastralManagerHealthResult, integrity_flags),
        16
    );
    assert_eq!(
        mem::offset_of!(PastralManagerHealthResult, server_process_id),
        20
    );
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, session_id), 24);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, reserved0), 28);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, connect_us), 32);
    assert_eq!(
        mem::offset_of!(PastralManagerHealthResult, handshake_us),
        40
    );
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, health_us), 48);
    assert_eq!(mem::offset_of!(PastralManagerHealthResult, reserved1), 56);

    assert_eq!(PastralManagerHealthStatus::Connected as u32, 0);
    assert_eq!(PastralManagerHealthStatus::Disconnected as u32, 1);
    assert_eq!(PastralManagerHealthStatus::Timeout as u32, 2);
    assert_eq!(PastralManagerHealthStatus::ProtocolMismatch as u32, 3);
    assert_eq!(PastralManagerHealthStatus::AuthenticationFailed as u32, 4);
    assert_eq!(PastralManagerHealthStatus::Unhealthy as u32, 5);
    assert_eq!(PastralManagerHealthStatus::InvalidArgument as u32, 6);
    assert_eq!(PastralManagerHealthStatus::InternalError as u32, 7);
    assert_eq!(PastralManagerHealthStatus::AbiMismatch as u32, 8);

    assert_eq!(pastral_manager_ipc_abi_version(), 1);
    assert_eq!(pastral_manager_ipc_result_size(), 64);
}

#[test]
fn repository_header_matches_the_rust_contract() {
    let header = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("include/pastral_manager_ipc_bridge.h"),
    )
    .unwrap();
    for marker in [
        "PASTRAL_MANAGER_IPC_ABI_VERSION 1u",
        "PASTRAL_MANAGER_IPC_RESULT_BYTES 64u",
        "PASTRAL_MANAGER_STATUS_CONNECTED 0u",
        "PASTRAL_MANAGER_STATUS_ABI_MISMATCH 8u",
        "static_assert(sizeof(PastralManagerHealthResult) == 64",
        "pastral_manager_ipc_health_w",
    ] {
        assert!(header.contains(marker), "missing header marker: {marker}");
    }
}
