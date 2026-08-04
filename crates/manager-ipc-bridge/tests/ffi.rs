#![cfg(windows)]

use std::{
    fs, num::NonZeroUsize, os::windows::ffi::OsStrExt, path::PathBuf, thread, time::Duration,
};

use pastral_agent::{HealthServerConfig, serve_health};
use pastral_domain::ClipEventId;
use pastral_ipc_win::{current_token_identity, load_or_create_transport_material};
use pastral_manager_ipc_bridge::{
    PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK, PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK,
    PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES, PastralManagerHealthResult,
    PastralManagerHealthStatus, pastral_manager_ipc_health_w,
};

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

#[test]
fn null_result_returns_invalid_argument_without_dereference() {
    let code =
        unsafe { pastral_manager_ipc_health_w(std::ptr::null(), 0, 100, std::ptr::null_mut()) };
    assert_eq!(code, PastralManagerHealthStatus::InvalidArgument as i32);
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
