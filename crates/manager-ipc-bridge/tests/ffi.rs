#![cfg(windows)]

use std::{
    fs, num::NonZeroUsize, os::windows::ffi::OsStrExt, path::PathBuf, thread, time::Duration,
};

use pastral_agent::{
    DiagnosticStoragePolicy, HealthServerConfig, diagnostic_storage_limits, serve_health,
    serve_read,
};
use pastral_domain::{
    CaptureOrder, ClipEvent, ClipEventId, ClipRepresentation, ClipRepresentationId,
    ClipboardFormatIdentity, Fidelity, ProfileId, ProtectionDomain, ProtectionDomainId, RawDigest,
    StandardFormatId, UtcUnixMicros,
};
use pastral_ipc_win::{current_token_identity, load_or_create_transport_material};
use pastral_manager_ipc_bridge::{
    PASTRAL_MANAGER_CLIP_ITEM_BYTES, PASTRAL_MANAGER_CLIP_KIND_TEXT,
    PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE, PASTRAL_MANAGER_CLIP_UNAVAILABLE,
    PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK, PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK,
    PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES,
    PASTRAL_MANAGER_READ_ABI_VERSION, PASTRAL_MANAGER_READ_RESULT_BYTES, PastralManagerClipItem,
    PastralManagerHealthResult, PastralManagerHealthStatus, PastralManagerReadResult,
    pastral_manager_ipc_health_w, pastral_manager_ipc_history_w, pastral_manager_ipc_search_w,
};
use pastral_storage::{ClipCommit, RepresentationPayload, SearchProjection, Storage};

fn initialized_result() -> PastralManagerHealthResult {
    PastralManagerHealthResult {
        abi_version: PASTRAL_MANAGER_IPC_ABI_VERSION,
        struct_size: PASTRAL_MANAGER_IPC_RESULT_BYTES,
        status: u32::MAX,
        storage_schema_version: u32::MAX,
        integrity_flags: u32::MAX,
        server_process_id: u32::MAX,
        session_id: u32::MAX,
        reserved0: u32::MAX,
        connect_us: u64::MAX,
        handshake_us: u64::MAX,
        health_us: u64::MAX,
        reserved1: u64::MAX,
    }
}

fn initialized_read_result() -> PastralManagerReadResult {
    PastralManagerReadResult {
        abi_version: PASTRAL_MANAGER_READ_ABI_VERSION,
        struct_size: PASTRAL_MANAGER_READ_RESULT_BYTES,
        status: u32::MAX,
        item_count: u32::MAX,
        has_more: u32::MAX,
        required_item_capacity: u32::MAX,
        required_text_capacity: u32::MAX,
        server_process_id: u32::MAX,
        session_id: u32::MAX,
        reserved0: u32::MAX,
        connect_us: u64::MAX,
        handshake_us: u64::MAX,
        request_us: u64::MAX,
    }
}

fn sentinel_item() -> PastralManagerClipItem {
    PastralManagerClipItem {
        event_id: [0xa5; 16],
        capture_order: u64::MAX,
        observed_at_unix_micros: i64::MAX,
        kind: u32::MAX,
        flags: u32::MAX,
        preview_offset: u32::MAX,
        preview_length: u32::MAX,
        source_offset: u32::MAX,
        source_length: u32::MAX,
        reserved0: u32::MAX,
        reserved1: u32::MAX,
    }
}

fn missing_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "pastral-manager-ffi-missing-{}",
        ClipEventId::new_v4()
    ))
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-manager-ffi-connected-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wide(path: &std::path::Path) -> Vec<u16> {
    path.as_os_str().encode_wide().collect()
}

fn wide_text(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn commit_text(
    storage: &mut Storage<DiagnosticStoragePolicy>,
    domain: ProtectionDomain,
    order: u64,
    projection: Option<&str>,
) -> ClipEventId {
    let bytes = format!("payload-{order}").into_bytes();
    let digest = RawDigest::sha256_raw_v1(domain, &bytes).unwrap();
    let representation = ClipRepresentation::new(
        ClipRepresentationId::new_v4(),
        ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
        domain,
        bytes.len() as u64,
        Some(digest),
        Fidelity::FullFidelity,
    )
    .unwrap();
    let event_id = ClipEventId::new_v4();
    let event = ClipEvent::new(
        event_id,
        UtcUnixMicros::new(1_700_000_000_000_000 + order as i64).unwrap(),
        CaptureOrder::new(order).unwrap(),
        ProfileId::new_v4(),
        domain,
        vec![representation.clone()],
    )
    .unwrap();
    let payload = RepresentationPayload::new(representation.id(), bytes);
    let projection = projection
        .map(|value| SearchProjection::new(value.to_owned(), diagnostic_storage_limits()).unwrap());
    storage
        .commit_clip(ClipCommit::new(event, vec![payload], projection))
        .unwrap();
    event_id
}

#[test]
fn null_result_returns_invalid_argument_without_dereference() {
    let code =
        unsafe { pastral_manager_ipc_health_w(std::ptr::null(), 0, 100, std::ptr::null_mut()) };
    assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
}

#[test]
fn read_contract_rejects_invalid_arguments_without_writing_output_buffers() {
    let root = missing_root();
    let root_wide = wide(&root);
    let original_items = [sentinel_item(); 2];
    let original_text = [0xa5; 16];

    let mut result = initialized_read_result();
    let code = unsafe {
        pastral_manager_ipc_history_w(
            root_wide.as_ptr(),
            root_wide.len(),
            100,
            1,
            0,
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(result),
        )
    };
    assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
    assert_eq!(
        result.status,
        PastralManagerHealthStatus::InvalidArgument as u32
    );
    assert_eq!(result.item_count, 0);
    assert_eq!(result.required_item_capacity, 0);
    assert_eq!(result.required_text_capacity, 0);
    assert_eq!(result.reserved0, 0);

    let mut items = original_items;
    let mut text = original_text;
    for (timeout, limit, item_capacity, text_capacity) in [
        (49, 1, 2, 16),
        (5_001, 1, 2, 16),
        (100, 0, 2, 16),
        (100, 101, 2, 16),
        (100, 1, 101, 16),
        (100, 1, 2, 262_145),
    ] {
        let mut result = initialized_read_result();
        let code = unsafe {
            pastral_manager_ipc_history_w(
                root_wide.as_ptr(),
                root_wide.len(),
                timeout,
                limit,
                0,
                items.as_mut_ptr(),
                item_capacity,
                text.as_mut_ptr(),
                text_capacity,
                std::ptr::addr_of_mut!(result),
            )
        };
        assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
        assert_eq!(items, original_items);
        assert_eq!(text, original_text);
    }

    let mut result = initialized_read_result();
    result.abi_version += 1;
    let code = unsafe {
        pastral_manager_ipc_history_w(
            root_wide.as_ptr(),
            root_wide.len(),
            100,
            1,
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(result),
        )
    };
    assert_eq!(code, PastralManagerHealthStatus::AbiMismatch as i32);
    assert_eq!(
        result.status,
        PastralManagerHealthStatus::AbiMismatch as u32
    );
    assert_eq!(result.abi_version, PASTRAL_MANAGER_READ_ABI_VERSION);
    assert_eq!(result.struct_size, PASTRAL_MANAGER_READ_RESULT_BYTES);

    let query = wide_text("alpha");
    let mut result = initialized_read_result();
    let code = unsafe {
        pastral_manager_ipc_search_w(
            root_wide.as_ptr(),
            root_wide.len(),
            std::ptr::null(),
            query.len(),
            100,
            1,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(result),
        )
    };
    assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
}

#[test]
fn read_contract_sizes_and_fills_authenticated_history_and_search() {
    assert_eq!(
        std::mem::size_of::<PastralManagerClipItem>() as u32,
        PASTRAL_MANAGER_CLIP_ITEM_BYTES
    );
    let root = TestRoot::new();
    let mut storage = Storage::open(
        root.path().join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .unwrap();
    let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
    let _first = commit_text(&mut storage, domain, 1, Some("alpha beta"));
    let second = commit_text(&mut storage, domain, 2, Some("alpha OR beta"));
    let third = commit_text(&mut storage, domain, 3, None);
    drop(storage);

    let _material = load_or_create_transport_material(root.path()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::new(4).unwrap(),
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap()
        .without_summary();
        serve_read(config, &mut Vec::new()).unwrap();
    });

    let root_wide = wide(root.path());
    let mut history_result = initialized_read_result();
    let history_code = unsafe {
        pastral_manager_ipc_history_w(
            root_wide.as_ptr(),
            root_wide.len(),
            2_000,
            2,
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(history_result),
        )
    };
    assert_eq!(
        history_code,
        PastralManagerHealthStatus::InsufficientBuffer as i32
    );
    assert_eq!(history_result.required_item_capacity, 2);
    assert_eq!(history_result.required_text_capacity, 13);

    let mut history_items = vec![sentinel_item(); 3];
    let mut history_text = vec![0xa5; history_result.required_text_capacity as usize + 8];
    let history_code = unsafe {
        pastral_manager_ipc_history_w(
            root_wide.as_ptr(),
            root_wide.len(),
            2_000,
            2,
            0,
            history_items.as_mut_ptr(),
            history_result.required_item_capacity,
            history_text.as_mut_ptr(),
            history_result.required_text_capacity,
            std::ptr::addr_of_mut!(history_result),
        )
    };
    assert_eq!(history_code, PastralManagerHealthStatus::Connected as i32);
    assert_eq!(history_result.item_count, 2);
    assert_eq!(history_result.has_more, 1);
    assert_eq!(history_items[0].event_id, *third.as_uuid().as_bytes());
    assert_eq!(history_items[0].kind, PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE);
    assert_eq!(history_items[0].flags, PASTRAL_MANAGER_CLIP_UNAVAILABLE);
    assert_eq!(history_items[0].preview_length, 0);
    assert_eq!(history_items[1].event_id, *second.as_uuid().as_bytes());
    assert_eq!(history_items[1].kind, PASTRAL_MANAGER_CLIP_KIND_TEXT);
    assert_eq!(history_items[1].preview_offset, 0);
    assert_eq!(history_items[1].preview_length, 13);
    assert_eq!(&history_text[..13], b"alpha OR beta");
    assert_eq!(history_items[2], sentinel_item());
    assert!(history_text[13..].iter().all(|value| *value == 0xa5));

    let query = wide_text("alpha OR");
    let mut search_result = initialized_read_result();
    let search_code = unsafe {
        pastral_manager_ipc_search_w(
            root_wide.as_ptr(),
            root_wide.len(),
            query.as_ptr(),
            query.len(),
            2_000,
            10,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(search_result),
        )
    };
    assert_eq!(
        search_code,
        PastralManagerHealthStatus::InsufficientBuffer as i32
    );
    assert_eq!(search_result.required_item_capacity, 1);
    assert_eq!(search_result.required_text_capacity, 13);

    let mut search_items = vec![sentinel_item(); 2];
    let mut search_text = vec![0xa5; search_result.required_text_capacity as usize + 8];
    let search_code = unsafe {
        pastral_manager_ipc_search_w(
            root_wide.as_ptr(),
            root_wide.len(),
            query.as_ptr(),
            query.len(),
            2_000,
            10,
            search_items.as_mut_ptr(),
            search_result.required_item_capacity,
            search_text.as_mut_ptr(),
            search_result.required_text_capacity,
            std::ptr::addr_of_mut!(search_result),
        )
    };
    assert_eq!(search_code, PastralManagerHealthStatus::Connected as i32);
    assert_eq!(search_result.item_count, 1);
    assert_eq!(search_result.has_more, 0);
    assert_eq!(search_items[0].event_id, *second.as_uuid().as_bytes());
    assert_eq!(&search_text[..13], b"alpha OR beta");
    assert_eq!(search_items[1], sentinel_item());
    assert!(search_text[13..].iter().all(|value| *value == 0xa5));
    server.join().unwrap();
}

#[test]
fn invalid_path_timeout_and_utf16_fail_closed_with_initialized_output() {
    let root = missing_root();
    let root_wide = wide(&root);
    let invalid_cases = [
        (std::ptr::null(), 1usize, 100u32),
        (root_wide.as_ptr(), 0, 100),
        (root_wide.as_ptr(), 32_768, 100),
        (root_wide.as_ptr(), root_wide.len(), 49),
        (root_wide.as_ptr(), root_wide.len(), 5_001),
    ];

    for (path, length, timeout) in invalid_cases {
        let mut result = initialized_result();
        let code = unsafe {
            pastral_manager_ipc_health_w(path, length, timeout, std::ptr::addr_of_mut!(result))
        };
        assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
        assert_eq!(
            result.status,
            PastralManagerHealthStatus::InvalidArgument as u32
        );
        assert_eq!(result.abi_version, PASTRAL_MANAGER_IPC_ABI_VERSION);
        assert_eq!(result.struct_size, PASTRAL_MANAGER_IPC_RESULT_BYTES);
        assert_eq!(result.reserved0, 0);
        assert_eq!(result.reserved1, 0);
    }

    for invalid_utf16 in [vec![0], vec![0xd800]] {
        let mut result = initialized_result();
        let code = unsafe {
            pastral_manager_ipc_health_w(
                invalid_utf16.as_ptr(),
                invalid_utf16.len(),
                100,
                std::ptr::addr_of_mut!(result),
            )
        };
        assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
        assert_eq!(
            result.status,
            PastralManagerHealthStatus::InvalidArgument as u32
        );
    }
}

#[test]
fn abi_header_mismatch_is_rejected_and_normalized() {
    for (abi_version, struct_size) in [(2, 64), (1, 63)] {
        let root = missing_root();
        let root_wide = wide(&root);
        let mut result = initialized_result();
        result.abi_version = abi_version;
        result.struct_size = struct_size;

        let code = unsafe {
            pastral_manager_ipc_health_w(
                root_wide.as_ptr(),
                root_wide.len(),
                100,
                std::ptr::addr_of_mut!(result),
            )
        };
        assert_eq!(code, PastralManagerHealthStatus::AbiMismatch as i32);
        assert_eq!(
            result.status,
            PastralManagerHealthStatus::AbiMismatch as u32
        );
        assert_eq!(result.abi_version, PASTRAL_MANAGER_IPC_ABI_VERSION);
        assert_eq!(result.struct_size, PASTRAL_MANAGER_IPC_RESULT_BYTES);
    }
}

#[test]
fn real_agent_health_round_trips_through_the_c_contract() {
    let root = TestRoot::new();
    let _material = load_or_create_transport_material(root.path()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::MIN,
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap()
        .without_summary();
        serve_health(config, &mut Vec::new()).unwrap();
    });
    let root_wide = wide(root.path());
    let mut result = initialized_result();

    let code = unsafe {
        pastral_manager_ipc_health_w(
            root_wide.as_ptr(),
            root_wide.len(),
            2_000,
            std::ptr::addr_of_mut!(result),
        )
    };
    assert_eq!(code, PastralManagerHealthStatus::Connected as i32);
    assert_eq!(result.status, PastralManagerHealthStatus::Connected as u32);
    assert_eq!(result.storage_schema_version, 1);
    assert_eq!(
        result.integrity_flags
            & (PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK
                | PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK),
        PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK | PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK
    );
    assert_ne!(result.server_process_id, 0);
    assert_eq!(
        result.session_id,
        current_token_identity().unwrap().session_id()
    );
    assert_eq!(result.reserved0, 0);
    assert_eq!(result.reserved1, 0);
    server.join().unwrap();
}

#[test]
fn valid_missing_agent_returns_disconnected_and_creates_nothing() {
    let root = missing_root();
    let root_wide = wide(&root);
    let mut result = initialized_result();

    let code = unsafe {
        pastral_manager_ipc_health_w(
            root_wide.as_ptr(),
            root_wide.len(),
            100,
            std::ptr::addr_of_mut!(result),
        )
    };
    assert_eq!(code, PastralManagerHealthStatus::Disconnected as i32);
    assert_eq!(
        result.status,
        PastralManagerHealthStatus::Disconnected as u32
    );
    assert_eq!(result.storage_schema_version, 0);
    assert_eq!(result.integrity_flags, 0);
    assert_eq!(result.server_process_id, 0);
    assert_eq!(result.session_id, 0);
    assert_eq!(result.reserved0, 0);
    assert_eq!(result.reserved1, 0);
    assert!(!root.exists());
}
