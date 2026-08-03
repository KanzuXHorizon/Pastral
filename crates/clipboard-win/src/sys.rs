use std::{
    cell::RefCell,
    ffi::c_void,
    ptr,
    sync::{
        OnceLock,
        mpsc::{SyncSender, TrySendError},
    },
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HGLOBAL, HWND, SetLastError,
    },
    System::{
        DataExchange::{
            AddClipboardFormatListener, CloseClipboard, EnumClipboardFormats, GetClipboardData,
            GetClipboardFormatNameW, GetClipboardSequenceNumber, IsClipboardFormatAvailable,
            OpenClipboard, RemoveClipboardFormatListener,
        },
        LibraryLoader::GetModuleHandleW,
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        Threading::GetCurrentThreadId,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        HWND_MESSAGE, MSG, PostMessageW, PostQuitMessage, PostThreadMessageW, RegisterClassW,
        TranslateMessage, WM_APP, WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSW,
    },
};

#[cfg(test)]
use windows_sys::Win32::{
    Foundation::GlobalFree,
    System::Memory::{GMEM_MOVEABLE, GlobalAlloc},
};

use crate::{ClipboardError, ClipboardNotification, ClipboardSequence};

const LISTENER_STOP_MESSAGE: u32 = WM_APP + 0x4A1;
const MAX_REGISTERED_NAME_UNITS: usize = 32_768;

thread_local! {
    static LISTENER_SENDER: RefCell<Option<SyncSender<ClipboardNotification>>> = const { RefCell::new(None) };
}

static CLASS_NAME: OnceLock<Box<[u16]>> = OnceLock::new();
static CLASS_REGISTRATION: OnceLock<Result<(), ClipboardError>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeGlobalHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ListenerEndpoint {
    window_handle: usize,
    thread_id: u32,
}

impl NativeGlobalHandle {
    fn from_raw(raw: *mut c_void) -> Option<Self> {
        (!raw.is_null()).then_some(Self(raw as usize))
    }

    fn as_raw(self) -> HGLOBAL {
        self.0 as HGLOBAL
    }
}

pub(crate) fn open_clipboard() -> Result<(), ClipboardError> {
    // SAFETY: A null owner window is permitted for read-only clipboard access. No Rust references
    // cross the call and successful ownership is paired with `close_clipboard` by ClipboardSession.
    let result = unsafe { OpenClipboard(ptr::null_mut()) };
    if result == 0 {
        return Err(ClipboardError::ClipboardUnavailable);
    }
    Ok(())
}

pub(crate) fn close_clipboard() -> Result<(), ClipboardError> {
    // SAFETY: Called exactly once for a successfully opened clipboard on the same thread.
    let result = unsafe { CloseClipboard() };
    if result == 0 {
        return Err(last_error("CloseClipboard"));
    }
    Ok(())
}

pub(crate) fn clipboard_sequence() -> ClipboardSequence {
    // SAFETY: GetClipboardSequenceNumber has no pointer parameters or ownership effects.
    ClipboardSequence::from_raw(unsafe { GetClipboardSequenceNumber() })
}

pub(crate) fn enumerate_next(previous: u32) -> Result<Option<u32>, ClipboardError> {
    // SAFETY: SetLastError sets thread-local Win32 error state before the following enumeration call.
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: ClipboardSession guarantees the clipboard is open on this thread. `previous` is zero
    // to begin enumeration or a value returned by the previous call.
    let next = unsafe { EnumClipboardFormats(previous) };
    if next != 0 {
        return Ok(Some(next));
    }
    // SAFETY: Reads the calling thread's Win32 last-error value immediately after enumeration.
    let code = unsafe { GetLastError() };
    if code == ERROR_SUCCESS {
        Ok(None)
    } else {
        Err(ClipboardError::win32("EnumClipboardFormats", code))
    }
}

pub(crate) fn registered_format_name(format: u32) -> Result<String, ClipboardError> {
    let mut capacity = 256usize;
    loop {
        let cch = i32::try_from(capacity).map_err(|_| ClipboardError::LengthOutOfRange)?;
        let mut buffer = vec![0u16; capacity];
        // SAFETY: `buffer` is writable for `capacity` UTF-16 units and remains alive for the call.
        // The format is classified in the registered range before this function is called.
        let copied = unsafe { GetClipboardFormatNameW(format, buffer.as_mut_ptr(), cch) };
        if copied == 0 {
            return Err(last_error("GetClipboardFormatNameW"));
        }
        let copied = usize::try_from(copied).map_err(|_| ClipboardError::LengthOutOfRange)?;
        if copied + 1 < capacity {
            return String::from_utf16(&buffer[..copied])
                .map_err(|_| ClipboardError::RegisteredNameInvalidUtf16);
        }
        if capacity >= MAX_REGISTERED_NAME_UNITS {
            return Err(ClipboardError::RegisteredNameTooLong);
        }
        capacity = (capacity * 2).min(MAX_REGISTERED_NAME_UNITS);
    }
}

pub(crate) fn is_format_available(format: u32) -> bool {
    // SAFETY: IsClipboardFormatAvailable has no pointers and does not transfer ownership.
    unsafe { IsClipboardFormatAvailable(format) != 0 }
}

pub(crate) fn clipboard_data_handle(format: u32) -> Result<NativeGlobalHandle, ClipboardError> {
    // SAFETY: ClipboardSession guarantees the clipboard is open. The returned handle remains owned
    // by the clipboard; this function converts it to an opaque token and never frees it.
    let handle = unsafe { GetClipboardData(format) };
    NativeGlobalHandle::from_raw(handle).ok_or_else(|| {
        let error = current_error();
        if error == ERROR_SUCCESS {
            ClipboardError::DataHandleUnavailable
        } else {
            ClipboardError::win32("GetClipboardData", error)
        }
    })
}

pub(crate) fn copy_global(
    handle: NativeGlobalHandle,
    max_bytes: usize,
) -> Result<Vec<u8>, ClipboardError> {
    let raw = handle.as_raw();
    // SAFETY: `raw` is a non-null HGLOBAL token returned by a Win32 clipboard/global-memory API.
    let size = unsafe { GlobalSize(raw) };
    if size == 0 {
        return Err(ClipboardError::GlobalAllocationEmpty);
    }
    if size > max_bytes {
        return Err(ClipboardError::GlobalAllocationTooLarge {
            size,
            limit: max_bytes,
        });
    }
    // SAFETY: GlobalLock is called with the validated HGLOBAL. The pointer is used only while the
    // lock is held and for exactly `size` bytes reported by GlobalSize.
    let pointer = unsafe { GlobalLock(raw) };
    if pointer.is_null() {
        return Err(ClipboardError::GlobalLockFailed);
    }
    // SAFETY: `pointer` is valid for `size` bytes until GlobalUnlock; copying creates owned bytes and
    // no Rust reference outlives the lock.
    let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size) }.to_vec();

    // SAFETY: Resets the thread-local error so a false GlobalUnlock result can be distinguished from
    // failure when the lock count legitimately reaches zero.
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: Balances the successful GlobalLock above and does not free clipboard-owned memory.
    let unlock_result = unsafe { GlobalUnlock(raw) };
    if unlock_result == 0 {
        // SAFETY: Reads the thread-local error immediately after GlobalUnlock.
        let code = unsafe { GetLastError() };
        if code != ERROR_SUCCESS {
            return Err(ClipboardError::GlobalUnlockFailed);
        }
    }
    Ok(bytes)
}

pub(crate) fn run_listener(
    notifications: SyncSender<ClipboardNotification>,
    startup: SyncSender<Result<ListenerEndpoint, ClipboardError>>,
) -> Result<(), ClipboardError> {
    LISTENER_SENDER.with(|slot| {
        *slot.borrow_mut() = Some(notifications);
    });

    let result = run_listener_inner(startup);
    LISTENER_SENDER.with(|slot| {
        *slot.borrow_mut() = None;
    });
    result
}

fn run_listener_inner(
    startup: SyncSender<Result<ListenerEndpoint, ClipboardError>>,
) -> Result<(), ClipboardError> {
    if let Err(error) = register_listener_class() {
        let _ = startup.send(Err(error.clone()));
        return Err(error);
    }
    let class_name = class_name();
    // SAFETY: Null requests the current executable module handle and has no ownership transfer.
    let module = unsafe { GetModuleHandleW(ptr::null()) };
    if module.is_null() {
        let error = last_error("GetModuleHandleW");
        let _ = startup.send(Err(error.clone()));
        return Err(error);
    }

    // SAFETY: The registered class name and module remain valid for process lifetime. HWND_MESSAGE
    // creates a message-only window. No creation parameter or raw Rust pointer is passed.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name,
            class_name,
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            ptr::null_mut(),
            module.cast(),
            ptr::null(),
        )
    };
    if hwnd.is_null() {
        let error = last_error("CreateWindowExW");
        let _ = startup.send(Err(error.clone()));
        return Err(error);
    }

    // SAFETY: `hwnd` is a valid message-only window owned by this thread.
    if unsafe { AddClipboardFormatListener(hwnd) } == 0 {
        let error = last_error("AddClipboardFormatListener");
        // SAFETY: Destroys the just-created window on its owning thread after listener registration
        // failed; no other code can observe the handle because startup has not succeeded.
        unsafe { DestroyWindow(hwnd) };
        let _ = startup.send(Err(error.clone()));
        return Err(error);
    }

    // SAFETY: Reads the current listener thread identifier and has no ownership effects.
    let thread_id = unsafe { GetCurrentThreadId() };
    if startup
        .send(Ok(ListenerEndpoint {
            window_handle: hwnd as usize,
            thread_id,
        }))
        .is_err()
    {
        // SAFETY: Removes the listener and destroys the window on its owning thread when the caller
        // abandoned startup.
        unsafe {
            RemoveClipboardFormatListener(hwnd);
            DestroyWindow(hwnd);
        }
        return Err(ClipboardError::ListenerStartupClosed);
    }

    let mut message = MSG::default();
    loop {
        // SAFETY: `message` is writable and valid for the call; null HWND reads this thread's queue.
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result == -1 {
            // SAFETY: Best-effort cleanup on the owning thread after message-loop failure.
            unsafe {
                RemoveClipboardFormatListener(hwnd);
                DestroyWindow(hwnd);
            }
            return Err(last_error("GetMessageW"));
        }
        if result == 0 {
            break;
        }
        if message.hwnd.is_null() && message.message == LISTENER_STOP_MESSAGE {
            // SAFETY: This thread-message fallback executes on the owning listener thread and
            // releases the same listener/window resources as the HWND stop-message path.
            unsafe {
                RemoveClipboardFormatListener(hwnd);
                DestroyWindow(hwnd);
            }
            continue;
        }
        // SAFETY: `message` was initialized by GetMessageW and remains valid for dispatch.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

pub(crate) fn post_listener_stop(endpoint: ListenerEndpoint) -> Result<(), ClipboardError> {
    match post_message(
        endpoint.window_handle,
        LISTENER_STOP_MESSAGE,
        "PostMessageW(stop)",
    ) {
        Ok(()) => Ok(()),
        Err(window_error) => {
            // SAFETY: `thread_id` was captured from the listener thread after its message queue and
            // window were active. The private thread message carries no pointers or Rust references.
            if unsafe { PostThreadMessageW(endpoint.thread_id, LISTENER_STOP_MESSAGE, 0, 0) } == 0 {
                Err(window_error)
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn post_listener_test_update(
    endpoint: ListenerEndpoint,
) -> Result<(), ClipboardError> {
    post_message(
        endpoint.window_handle,
        WM_CLIPBOARDUPDATE,
        "PostMessageW(test update)",
    )
}

fn post_message(
    handle: usize,
    message: u32,
    operation: &'static str,
) -> Result<(), ClipboardError> {
    let hwnd = handle as HWND;
    if hwnd.is_null() {
        return Err(ClipboardError::win32(operation, ERROR_SUCCESS));
    }
    // SAFETY: The token was created from the listener HWND and remains valid until stop/join.
    if unsafe { PostMessageW(hwnd, message, 0, 0) } == 0 {
        return Err(last_error(operation));
    }
    Ok(())
}

fn register_listener_class() -> Result<(), ClipboardError> {
    CLASS_REGISTRATION
        .get_or_init(|| {
            // SAFETY: Null requests the current executable module handle.
            let module = unsafe { GetModuleHandleW(ptr::null()) };
            if module.is_null() {
                return Err(last_error("GetModuleHandleW"));
            }
            let class = WNDCLASSW {
                lpfnWndProc: Some(listener_window_proc),
                hInstance: module.cast(),
                lpszClassName: class_name(),
                ..WNDCLASSW::default()
            };
            // SAFETY: `class` points to a valid process-lifetime class name and callback.
            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 {
                // SAFETY: Reads last error immediately after RegisterClassW.
                let code = unsafe { GetLastError() };
                if code != ERROR_CLASS_ALREADY_EXISTS {
                    return Err(ClipboardError::win32("RegisterClassW", code));
                }
            }
            Ok(())
        })
        .clone()
}

fn class_name() -> *const u16 {
    CLASS_NAME
        .get_or_init(|| {
            "Pastral.Clipboard.Listener\0"
                .encode_utf16()
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .as_ptr()
}

unsafe extern "system" fn listener_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match message {
        WM_CLIPBOARDUPDATE => {
            let notification = ClipboardNotification::new(clipboard_sequence());
            LISTENER_SENDER.with(|slot| {
                if let Some(sender) = slot.borrow().as_ref() {
                    match sender.try_send(notification) {
                        Ok(())
                        | Err(TrySendError::Full(_))
                        | Err(TrySendError::Disconnected(_)) => {}
                    }
                }
            });
            0
        }
        LISTENER_STOP_MESSAGE => {
            // SAFETY: The callback runs on the window-owning listener thread. Removal and destruction
            // are paired with the successful registration/creation in `run_listener_inner`.
            unsafe {
                RemoveClipboardFormatListener(hwnd);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            // SAFETY: Posts WM_QUIT to the current listener thread after its window is destroyed.
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => {
            // SAFETY: Delegates unhandled messages using the original message parameters.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

fn current_error() -> u32 {
    // SAFETY: Reads the current thread's Win32 last-error value.
    unsafe { GetLastError() }
}

fn last_error(operation: &'static str) -> ClipboardError {
    ClipboardError::win32(operation, current_error())
}

#[cfg(test)]
pub(crate) fn copy_synthetic_global(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<Vec<u8>, ClipboardError> {
    // SAFETY: Allocates a movable global-memory block solely for this test helper. The handle is
    // always freed below and never transferred to the clipboard.
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    let Some(handle) = NativeGlobalHandle::from_raw(handle) else {
        return Err(last_error("GlobalAlloc"));
    };
    if !bytes.is_empty() {
        // SAFETY: The allocation is at least `bytes.len()` bytes and is locked only for the copy.
        let pointer = unsafe { GlobalLock(handle.as_raw()) };
        if pointer.is_null() {
            // SAFETY: Frees the owned test allocation after lock failure.
            unsafe { GlobalFree(handle.as_raw()) };
            return Err(ClipboardError::GlobalLockFailed);
        }
        // SAFETY: Source and destination are valid for `bytes.len()` and non-overlapping.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len()) };
        // SAFETY: Balances the test helper GlobalLock.
        unsafe { GlobalUnlock(handle.as_raw()) };
    }
    let result = copy_global(handle, max_bytes);
    // SAFETY: Frees the owned synthetic allocation after no references or locks remain.
    unsafe { GlobalFree(handle.as_raw()) };
    result
}
