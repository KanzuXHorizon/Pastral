use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr, slice,
    time::Duration,
};

use crate::{
    ManagerHealthSnapshot, PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES,
    PastralManagerHealthResult, PastralManagerHealthStatus, query_health,
};

const MIN_TIMEOUT_MS: u32 = 50;
const MAX_TIMEOUT_MS: u32 = 5_000;
const MAX_DATA_ROOT_UTF16_UNITS: usize = 32_767;

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_abi_version() -> u32 {
    PASTRAL_MANAGER_IPC_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_result_size() -> u32 {
    PASTRAL_MANAGER_IPC_RESULT_BYTES
}

/// Performs one bounded authenticated Health query through the manager IPC bridge.
///
/// # Safety
///
/// `result` must point to writable storage for one `PastralManagerHealthResult` whose
/// `abi_version` and `struct_size` fields are initialized to the exported constants.
/// When `data_root_length` is nonzero, `data_root` must point to that many readable
/// UTF-16 code units for the duration of the call. The pointed ranges must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pastral_manager_ipc_health_w(
    data_root: *const u16,
    data_root_length: usize,
    timeout_ms: u32,
    result: *mut PastralManagerHealthResult,
) -> i32 {
    if result.is_null() {
        return PastralManagerHealthStatus::InvalidArgument.as_i32();
    }

    let caller_header = unsafe { ((*result).abi_version, (*result).struct_size) };
    if caller_header
        != (
            PASTRAL_MANAGER_IPC_ABI_VERSION,
            PASTRAL_MANAGER_IPC_RESULT_BYTES,
        )
    {
        unsafe {
            ptr::write(
                result,
                PastralManagerHealthResult::failed(PastralManagerHealthStatus::AbiMismatch),
            );
        }
        return PastralManagerHealthStatus::AbiMismatch.as_i32();
    }

    unsafe {
        ptr::write(
            result,
            PastralManagerHealthResult::failed(PastralManagerHealthStatus::InvalidArgument),
        );
    }

    if data_root.is_null()
        || data_root_length == 0
        || data_root_length > MAX_DATA_ROOT_UTF16_UNITS
        || !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms)
    {
        return PastralManagerHealthStatus::InvalidArgument.as_i32();
    }

    let units = unsafe { slice::from_raw_parts(data_root, data_root_length) };
    if units.contains(&0) {
        return PastralManagerHealthStatus::InvalidArgument.as_i32();
    }
    let text = match String::from_utf16(units) {
        Ok(text) => text,
        Err(_) => return PastralManagerHealthStatus::InvalidArgument.as_i32(),
    };
    let path = PathBuf::from(text);
    if !path.is_absolute() {
        return PastralManagerHealthStatus::InvalidArgument.as_i32();
    }

    let output =
        invoke_health(|| query_health(&path, Duration::from_millis(u64::from(timeout_ms))));
    let code = output.status as i32;
    unsafe {
        ptr::write(result, output);
    }
    code
}

fn invoke_health(operation: impl FnOnce() -> ManagerHealthSnapshot) -> PastralManagerHealthResult {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(snapshot) => PastralManagerHealthResult::from_snapshot(snapshot),
        Err(_) => PastralManagerHealthResult::failed(PastralManagerHealthStatus::InternalError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_contained_as_internal_error() {
        let result = invoke_health(|| panic!("test panic"));
        assert_eq!(
            result.status,
            PastralManagerHealthStatus::InternalError as u32
        );
        assert_eq!(result.reserved0, 0);
        assert_eq!(result.reserved1, 0);
    }
}
