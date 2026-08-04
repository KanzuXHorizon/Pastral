use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr, slice,
    time::Duration,
};

use pastral_domain::CaptureOrder;

use crate::{
    ManagerClipKind, ManagerHealthSnapshot, ManagerReadPageSnapshot,
    PASTRAL_MANAGER_CLIP_ITEM_BYTES, PASTRAL_MANAGER_CLIP_KIND_TEXT,
    PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE, PASTRAL_MANAGER_CLIP_PINNED,
    PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED, PASTRAL_MANAGER_CLIP_UNAVAILABLE,
    PASTRAL_MANAGER_IPC_ABI_VERSION, PASTRAL_MANAGER_IPC_RESULT_BYTES,
    PASTRAL_MANAGER_READ_ABI_VERSION, PASTRAL_MANAGER_READ_RESULT_BYTES, PastralManagerClipItem,
    PastralManagerHealthResult, PastralManagerHealthStatus, PastralManagerReadResult, query_health,
    query_history, query_search,
};

const MIN_TIMEOUT_MS: u32 = 50;
const MAX_TIMEOUT_MS: u32 = 5_000;
const MAX_DATA_ROOT_UTF16_UNITS: usize = 32_767;
const MAX_QUERY_UTF16_UNITS: usize = 1_024;
const MAX_READ_ITEMS: u32 = 100;
const MAX_TEXT_BUFFER_BYTES: u32 = 256 * 1_024;

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_abi_version() -> u32 {
    PASTRAL_MANAGER_IPC_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_result_size() -> u32 {
    PASTRAL_MANAGER_IPC_RESULT_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_read_abi_version() -> u32 {
    PASTRAL_MANAGER_READ_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_read_result_size() -> u32 {
    PASTRAL_MANAGER_READ_RESULT_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn pastral_manager_ipc_clip_item_size() -> u32 {
    PASTRAL_MANAGER_CLIP_ITEM_BYTES
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

/// Performs one bounded authenticated History query through caller-owned output buffers.
///
/// # Safety
///
/// `result` must point to writable storage for one `PastralManagerReadResult` whose header is
/// initialized to the exported read constants. Pointer/capacity pairs must either both be zero/null
/// or reference writable nonoverlapping storage for the declared capacity. `data_root` must reference
/// readable UTF-16 code units for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pastral_manager_ipc_history_w(
    data_root: *const u16,
    data_root_length: usize,
    timeout_ms: u32,
    limit: u32,
    before_capture_order: u64,
    items: *mut PastralManagerClipItem,
    item_capacity: u32,
    text_buffer: *mut u8,
    text_capacity: u32,
    result: *mut PastralManagerReadResult,
) -> i32 {
    let Some(path) = (unsafe {
        validate_read_call(ReadCallArgs {
            data_root,
            data_root_length,
            timeout_ms,
            limit,
            items,
            item_capacity,
            text_buffer,
            text_capacity,
            result,
        })
    }) else {
        return unsafe { read_result_status(result) };
    };
    let cursor = if before_capture_order == 0 {
        None
    } else {
        match CaptureOrder::new(before_capture_order) {
            Ok(value) => Some(value),
            Err(_) => return unsafe { invalid_read_result(result) },
        }
    };

    let execution = invoke_read(
        || {
            query_history(
                &path,
                Duration::from_millis(u64::from(timeout_ms)),
                limit,
                cursor,
            )
        },
        item_capacity,
        text_capacity,
    );
    unsafe { finish_read_call(execution, items, text_buffer, result) }
}

/// Performs one bounded authenticated literal Search query through caller-owned output buffers.
///
/// # Safety
///
/// The same requirements as `pastral_manager_ipc_history_w` apply. `query` must additionally point
/// to `query_length` readable UTF-16 code units without embedded NUL values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pastral_manager_ipc_search_w(
    data_root: *const u16,
    data_root_length: usize,
    query: *const u16,
    query_length: usize,
    timeout_ms: u32,
    limit: u32,
    items: *mut PastralManagerClipItem,
    item_capacity: u32,
    text_buffer: *mut u8,
    text_capacity: u32,
    result: *mut PastralManagerReadResult,
) -> i32 {
    let Some(path) = (unsafe {
        validate_read_call(ReadCallArgs {
            data_root,
            data_root_length,
            timeout_ms,
            limit,
            items,
            item_capacity,
            text_buffer,
            text_capacity,
            result,
        })
    }) else {
        return unsafe { read_result_status(result) };
    };
    let Some(query) = (unsafe { parse_required_utf16(query, query_length, MAX_QUERY_UTF16_UNITS) })
    else {
        return unsafe { invalid_read_result(result) };
    };

    let execution = invoke_read(
        || {
            query_search(
                &path,
                Duration::from_millis(u64::from(timeout_ms)),
                &query,
                limit,
            )
        },
        item_capacity,
        text_capacity,
    );
    unsafe { finish_read_call(execution, items, text_buffer, result) }
}

struct ReadCallArgs {
    data_root: *const u16,
    data_root_length: usize,
    timeout_ms: u32,
    limit: u32,
    items: *mut PastralManagerClipItem,
    item_capacity: u32,
    text_buffer: *mut u8,
    text_capacity: u32,
    result: *mut PastralManagerReadResult,
}

unsafe fn validate_read_call(args: ReadCallArgs) -> Option<PathBuf> {
    let ReadCallArgs {
        data_root,
        data_root_length,
        timeout_ms,
        limit,
        items,
        item_capacity,
        text_buffer,
        text_capacity,
        result,
    } = args;
    if result.is_null() {
        return None;
    }
    let caller_header = unsafe { ((*result).abi_version, (*result).struct_size) };
    if caller_header
        != (
            PASTRAL_MANAGER_READ_ABI_VERSION,
            PASTRAL_MANAGER_READ_RESULT_BYTES,
        )
    {
        unsafe {
            ptr::write(
                result,
                PastralManagerReadResult::failed(PastralManagerHealthStatus::AbiMismatch),
            );
        }
        return None;
    }
    unsafe {
        ptr::write(
            result,
            PastralManagerReadResult::failed(PastralManagerHealthStatus::InvalidArgument),
        );
    }

    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms)
        || !(1..=MAX_READ_ITEMS).contains(&limit)
        || item_capacity > MAX_READ_ITEMS
        || text_capacity > MAX_TEXT_BUFFER_BYTES
        || items.is_null() != (item_capacity == 0)
        || text_buffer.is_null() != (text_capacity == 0)
    {
        return None;
    }

    unsafe { parse_path(data_root, data_root_length) }
}

unsafe fn parse_path(data_root: *const u16, data_root_length: usize) -> Option<PathBuf> {
    let text =
        unsafe { parse_required_utf16(data_root, data_root_length, MAX_DATA_ROOT_UTF16_UNITS) }?;
    let path = PathBuf::from(text);
    path.is_absolute().then_some(path)
}

unsafe fn parse_required_utf16(
    value: *const u16,
    length: usize,
    max_length: usize,
) -> Option<String> {
    if value.is_null() || length == 0 || length > max_length {
        return None;
    }
    let units = unsafe { slice::from_raw_parts(value, length) };
    if units.contains(&0) {
        return None;
    }
    String::from_utf16(units).ok()
}

unsafe fn read_result_status(result: *mut PastralManagerReadResult) -> i32 {
    if result.is_null() {
        PastralManagerHealthStatus::InvalidArgument.as_i32()
    } else {
        unsafe { (*result).status as i32 }
    }
}

unsafe fn invalid_read_result(result: *mut PastralManagerReadResult) -> i32 {
    unsafe {
        ptr::write(
            result,
            PastralManagerReadResult::failed(PastralManagerHealthStatus::InvalidArgument),
        );
    }
    PastralManagerHealthStatus::InvalidArgument.as_i32()
}

enum ReadExecution {
    Result(PastralManagerReadResult),
    Connected {
        result: PastralManagerReadResult,
        items: Vec<PastralManagerClipItem>,
        text: Vec<u8>,
    },
}

fn invoke_read(
    operation: impl FnOnce() -> Result<ManagerReadPageSnapshot, crate::ManagerHealthStatus>,
    item_capacity: u32,
    text_capacity: u32,
) -> ReadExecution {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(snapshot)) => build_read_execution(snapshot, item_capacity, text_capacity),
        Ok(Err(status)) => ReadExecution::Result(PastralManagerReadResult::failed(status.into())),
        Err(_) => ReadExecution::Result(PastralManagerReadResult::failed(
            PastralManagerHealthStatus::InternalError,
        )),
    }
}

fn build_read_execution(
    snapshot: ManagerReadPageSnapshot,
    item_capacity: u32,
    text_capacity: u32,
) -> ReadExecution {
    let Some((items, text)) = build_copy_plan(&snapshot) else {
        return ReadExecution::Result(PastralManagerReadResult::failed(
            PastralManagerHealthStatus::InternalError,
        ));
    };
    let required_items = match u32::try_from(items.len()) {
        Ok(value) => value,
        Err(_) => {
            return ReadExecution::Result(PastralManagerReadResult::failed(
                PastralManagerHealthStatus::InternalError,
            ));
        }
    };
    let required_text = match u32::try_from(text.len()) {
        Ok(value) => value,
        Err(_) => {
            return ReadExecution::Result(PastralManagerReadResult::failed(
                PastralManagerHealthStatus::InternalError,
            ));
        }
    };
    if required_items > item_capacity || required_text > text_capacity {
        return ReadExecution::Result(PastralManagerReadResult::insufficient_buffer(
            required_items,
            required_text,
        ));
    }

    let result = PastralManagerReadResult {
        abi_version: PASTRAL_MANAGER_READ_ABI_VERSION,
        struct_size: PASTRAL_MANAGER_READ_RESULT_BYTES,
        status: PastralManagerHealthStatus::Connected as u32,
        item_count: required_items,
        has_more: u32::from(snapshot.has_more()),
        required_item_capacity: required_items,
        required_text_capacity: required_text,
        server_process_id: snapshot.server_process_id(),
        session_id: snapshot.session_id(),
        reserved0: 0,
        connect_us: duration_micros(snapshot.connect_elapsed()),
        handshake_us: duration_micros(snapshot.handshake_elapsed()),
        request_us: duration_micros(snapshot.request_elapsed()),
    };
    ReadExecution::Connected {
        result,
        items,
        text,
    }
}

fn build_copy_plan(
    snapshot: &ManagerReadPageSnapshot,
) -> Option<(Vec<PastralManagerClipItem>, Vec<u8>)> {
    if snapshot.items().len() > MAX_READ_ITEMS as usize {
        return None;
    }
    let mut output_items = Vec::with_capacity(snapshot.items().len());
    let mut text = Vec::new();

    for item in snapshot.items() {
        let preview_offset = u32::try_from(text.len()).ok()?;
        let preview_bytes = item.preview().as_bytes();
        let preview_length = u32::try_from(preview_bytes.len()).ok()?;
        text.extend_from_slice(preview_bytes);

        let (source_offset, source_length) = match item.source_label() {
            Some(source) => {
                let offset = u32::try_from(text.len()).ok()?;
                let bytes = source.as_bytes();
                let length = u32::try_from(bytes.len()).ok()?;
                text.extend_from_slice(bytes);
                (offset, length)
            }
            None => (0, 0),
        };
        if text.len() > MAX_TEXT_BUFFER_BYTES as usize {
            return None;
        }

        let kind = match item.kind() {
            ManagerClipKind::Text => PASTRAL_MANAGER_CLIP_KIND_TEXT,
            ManagerClipKind::Unavailable => PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE,
        };
        let mut flags = 0u32;
        if item.pinned() {
            flags |= PASTRAL_MANAGER_CLIP_PINNED;
        }
        if item.unavailable() {
            flags |= PASTRAL_MANAGER_CLIP_UNAVAILABLE;
        }
        if item.preview_truncated() {
            flags |= PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED;
        }
        output_items.push(PastralManagerClipItem {
            event_id: *item.event_id().as_uuid().as_bytes(),
            capture_order: item.capture_order().get(),
            observed_at_unix_micros: item.observed_at().get(),
            kind,
            flags,
            preview_offset,
            preview_length,
            source_offset,
            source_length,
            reserved0: 0,
            reserved1: 0,
        });
    }

    Some((output_items, text))
}

unsafe fn finish_read_call(
    execution: ReadExecution,
    items: *mut PastralManagerClipItem,
    text_buffer: *mut u8,
    result: *mut PastralManagerReadResult,
) -> i32 {
    let output = match execution {
        ReadExecution::Result(output) => output,
        ReadExecution::Connected {
            result: output,
            items: planned_items,
            text,
        } => {
            if !planned_items.is_empty() {
                unsafe {
                    ptr::copy_nonoverlapping(planned_items.as_ptr(), items, planned_items.len());
                }
            }
            if !text.is_empty() {
                unsafe {
                    ptr::copy_nonoverlapping(text.as_ptr(), text_buffer, text.len());
                }
            }
            output
        }
    };
    let code = output.status as i32;
    unsafe {
        ptr::write(result, output);
    }
    code
}

fn duration_micros(value: Duration) -> u64 {
    u64::try_from(value.as_micros()).unwrap_or(u64::MAX)
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
