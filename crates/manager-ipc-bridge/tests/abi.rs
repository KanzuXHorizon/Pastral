#![cfg(windows)]

use std::{fs, mem};

use pastral_manager_ipc_bridge::{
    PASTRAL_MANAGER_CLIP_ITEM_BYTES, PASTRAL_MANAGER_CLIP_KIND_TEXT,
    PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE, PASTRAL_MANAGER_CLIP_PINNED,
    PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED, PASTRAL_MANAGER_CLIP_UNAVAILABLE,
    PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES,
    PASTRAL_MANAGER_READ_ABI_VERSION, PASTRAL_MANAGER_READ_RESULT_BYTES, PastralManagerClipItem,
    PastralManagerHealthResult, PastralManagerHealthStatus, PastralManagerReadResult,
    pastral_manager_ipc_abi_version, pastral_manager_ipc_clip_item_size,
    pastral_manager_ipc_read_abi_version, pastral_manager_ipc_read_result_size,
    pastral_manager_ipc_result_size,
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
fn read_result_and_item_layout_are_exact() {
    assert_eq!(PASTRAL_MANAGER_READ_ABI_VERSION, 1);
    assert_eq!(PASTRAL_MANAGER_READ_RESULT_BYTES, 64);
    assert_eq!(PASTRAL_MANAGER_CLIP_ITEM_BYTES, 64);

    assert_eq!(mem::size_of::<PastralManagerReadResult>(), 64);
    assert_eq!(mem::align_of::<PastralManagerReadResult>(), 8);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, abi_version), 0);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, struct_size), 4);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, status), 8);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, item_count), 12);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, has_more), 16);
    assert_eq!(
        mem::offset_of!(PastralManagerReadResult, required_item_capacity),
        20
    );
    assert_eq!(
        mem::offset_of!(PastralManagerReadResult, required_text_capacity),
        24
    );
    assert_eq!(
        mem::offset_of!(PastralManagerReadResult, server_process_id),
        28
    );
    assert_eq!(mem::offset_of!(PastralManagerReadResult, session_id), 32);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, reserved0), 36);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, connect_us), 40);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, handshake_us), 48);
    assert_eq!(mem::offset_of!(PastralManagerReadResult, request_us), 56);

    assert_eq!(mem::size_of::<PastralManagerClipItem>(), 64);
    assert_eq!(mem::align_of::<PastralManagerClipItem>(), 8);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, event_id), 0);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, capture_order), 16);
    assert_eq!(
        mem::offset_of!(PastralManagerClipItem, observed_at_unix_micros),
        24
    );
    assert_eq!(mem::offset_of!(PastralManagerClipItem, kind), 32);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, flags), 36);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, preview_offset), 40);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, preview_length), 44);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, source_offset), 48);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, source_length), 52);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, reserved0), 56);
    assert_eq!(mem::offset_of!(PastralManagerClipItem, reserved1), 60);

    assert_eq!(PastralManagerHealthStatus::InsufficientBuffer as u32, 9);
    assert_eq!(PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE, 0);
    assert_eq!(PASTRAL_MANAGER_CLIP_KIND_TEXT, 1);
    assert_eq!(PASTRAL_MANAGER_CLIP_PINNED, 1 << 0);
    assert_eq!(PASTRAL_MANAGER_CLIP_UNAVAILABLE, 1 << 1);
    assert_eq!(PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED, 1 << 2);

    assert_eq!(pastral_manager_ipc_read_abi_version(), 1);
    assert_eq!(pastral_manager_ipc_read_result_size(), 64);
    assert_eq!(pastral_manager_ipc_clip_item_size(), 64);
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
        "PASTRAL_MANAGER_READ_ABI_VERSION 1u",
        "PASTRAL_MANAGER_READ_RESULT_BYTES 64u",
        "PASTRAL_MANAGER_CLIP_ITEM_BYTES 64u",
        "PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER 9u",
        "PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE 0u",
        "PASTRAL_MANAGER_CLIP_KIND_TEXT 1u",
        "PASTRAL_MANAGER_CLIP_PINNED (1u << 0)",
        "PASTRAL_MANAGER_CLIP_UNAVAILABLE (1u << 1)",
        "PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED (1u << 2)",
        "static_assert(sizeof(PastralManagerReadResult) == 64",
        "static_assert(sizeof(PastralManagerClipItem) == 64",
        "pastral_manager_ipc_read_abi_version",
        "pastral_manager_ipc_read_result_size",
        "pastral_manager_ipc_clip_item_size",
        "pastral_manager_ipc_history_w",
        "pastral_manager_ipc_search_w",
    ] {
        assert!(header.contains(marker), "missing header marker: {marker}");
    }
}
