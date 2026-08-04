use core::{ffi::c_void, mem, ptr, slice};
use std::{os::windows::ffi::OsStrExt, path::Path};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_INSUFFICIENT_BUFFER,
        GetLastError, HANDLE, HLOCAL, LocalFree,
    },
    Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom, CRYPT_INTEGER_BLOB,
        CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
    Security::{
        GetLengthSid, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, IsValidSid,
        SID_AND_ATTRIBUTES, TOKEN_GROUPS, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TOKEN_USER,
        TokenGroups, TokenIntegrityLevel, TokenSessionId, TokenUser,
    },
    Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW},
    System::{
        SystemServices::{SE_GROUP_ENABLED, SE_GROUP_LOGON_ID},
        Threading::{
            GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};
use zeroize::Zeroizing;

use crate::{TokenIdentity, TransportError};

const MAX_TOKEN_INFORMATION_BYTES: usize = 64 * 1024;

pub(crate) fn fill_system_random(target: &mut [u8]) -> Result<(), TransportError> {
    let length = u32::try_from(target.len())
        .map_err(|_| TransportError::SizeLimit("random request exceeds u32"))?;
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            target.as_mut_ptr(),
            length,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(TransportError::NtStatus {
            operation: "BCryptGenRandom",
            status,
        });
    }
    Ok(())
}

pub(crate) fn protect_user_data(
    plaintext: &[u8],
    entropy: &[u8],
    maximum_output: usize,
) -> Result<Vec<u8>, TransportError> {
    let input = blob_from_slice(plaintext)?;
    let entropy = blob_from_slice(entropy)?;
    let mut output = CRYPT_INTEGER_BLOB::default();
    let succeeded = unsafe {
        CryptProtectData(
            &input,
            ptr::null(),
            &entropy,
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 {
        return Err(last_error("CryptProtectData"));
    }
    copy_and_release_blob(output, maximum_output, false)
}

pub(crate) fn unprotect_user_data(
    ciphertext: &[u8],
    entropy: &[u8],
    maximum_output: usize,
) -> Result<Zeroizing<Vec<u8>>, TransportError> {
    let input = blob_from_slice(ciphertext)?;
    let entropy = blob_from_slice(entropy)?;
    let mut output = CRYPT_INTEGER_BLOB::default();
    let succeeded = unsafe {
        CryptUnprotectData(
            &input,
            ptr::null_mut(),
            &entropy,
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 {
        return Err(last_error("CryptUnprotectData"));
    }
    copy_and_release_sensitive_blob(output, maximum_output)
}

fn blob_from_slice(value: &[u8]) -> Result<CRYPT_INTEGER_BLOB, TransportError> {
    Ok(CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(value.len())
            .map_err(|_| TransportError::SizeLimit("DPAPI input exceeds u32"))?,
        pbData: value.as_ptr().cast_mut(),
    })
}

fn copy_and_release_blob(
    blob: CRYPT_INTEGER_BLOB,
    maximum_output: usize,
    zero_before_release: bool,
) -> Result<Vec<u8>, TransportError> {
    let length = usize::try_from(blob.cbData)
        .map_err(|_| TransportError::SizeLimit("DPAPI output length is invalid"))?;
    if length == 0 || blob.pbData.is_null() {
        release_local(blob.pbData)?;
        return Err(TransportError::Windows {
            operation: "DPAPI returned empty output",
            code: 0,
        });
    }
    if length > maximum_output {
        if zero_before_release {
            unsafe { ptr::write_bytes(blob.pbData, 0, length) };
        }
        release_local(blob.pbData)?;
        return Err(TransportError::SizeLimit("DPAPI output exceeds bound"));
    }

    let bytes = unsafe { slice::from_raw_parts(blob.pbData, length) };
    let copied = bytes.to_vec();
    if zero_before_release {
        unsafe { ptr::write_bytes(blob.pbData, 0, length) };
    }
    release_local(blob.pbData)?;
    Ok(copied)
}

fn copy_and_release_sensitive_blob(
    blob: CRYPT_INTEGER_BLOB,
    maximum_output: usize,
) -> Result<Zeroizing<Vec<u8>>, TransportError> {
    let length = usize::try_from(blob.cbData)
        .map_err(|_| TransportError::SizeLimit("DPAPI output length is invalid"))?;
    if length == 0 || blob.pbData.is_null() {
        release_local(blob.pbData)?;
        return Err(TransportError::Windows {
            operation: "DPAPI returned empty output",
            code: 0,
        });
    }
    if length > maximum_output {
        unsafe { ptr::write_bytes(blob.pbData, 0, length) };
        release_local(blob.pbData)?;
        return Err(TransportError::SizeLimit("DPAPI output exceeds bound"));
    }

    let bytes = unsafe { slice::from_raw_parts(blob.pbData, length) };
    let copied = Zeroizing::new(bytes.to_vec());
    unsafe { ptr::write_bytes(blob.pbData, 0, length) };
    release_local(blob.pbData)?;
    Ok(copied)
}

fn release_local(pointer: *mut u8) -> Result<(), TransportError> {
    if pointer.is_null() {
        return Ok(());
    }
    let result = unsafe { LocalFree(pointer as HLOCAL) };
    if !result.is_null() {
        return Err(last_error("LocalFree"));
    }
    Ok(())
}

pub(crate) fn current_process_id() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub(crate) fn query_process_token_identity(
    process_id: u32,
) -> Result<TokenIdentity, TransportError> {
    if process_id == 0 {
        return Err(TransportError::InvalidTokenIdentity(
            "process ID must be nonzero",
        ));
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    let process = OwnedHandle::new(process, "OpenProcess")?;
    let mut token: HANDLE = ptr::null_mut();
    let opened = unsafe { OpenProcessToken(process.raw(), TOKEN_QUERY, &mut token) };
    if opened == 0 {
        return Err(last_error("OpenProcessToken"));
    }
    let token = OwnedHandle::new(token, "OpenProcessToken")?;

    let user_buffer = query_token_information(token.raw(), TokenUser)?;
    let user = unsafe { &*(user_buffer.as_ptr().cast::<TOKEN_USER>()) };
    let user_sid = copy_sid(user.User.Sid)?;

    let groups_buffer = query_token_information(token.raw(), TokenGroups)?;
    let logon_sid = extract_logon_sid(&groups_buffer)?;

    let session_buffer = query_token_information(token.raw(), TokenSessionId)?;
    if session_buffer.byte_len() < mem::size_of::<u32>() {
        return Err(TransportError::InvalidTokenIdentity(
            "token session buffer is truncated",
        ));
    }
    let session_id = unsafe { *(session_buffer.as_ptr().cast::<u32>()) };

    let integrity_buffer = query_token_information(token.raw(), TokenIntegrityLevel)?;
    let mandatory = unsafe { &*(integrity_buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()) };
    let integrity_rid = sid_last_sub_authority(mandatory.Label.Sid)?;

    TokenIdentity::new(user_sid, logon_sid, session_id, integrity_rid, process_id)
}

fn query_token_information(token: HANDLE, class: i32) -> Result<AlignedBuffer, TransportError> {
    let mut required = 0u32;
    let first = unsafe { GetTokenInformation(token, class, ptr::null_mut(), 0, &mut required) };
    if first != 0 || unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return Err(last_error("GetTokenInformation length"));
    }
    let required = usize::try_from(required)
        .map_err(|_| TransportError::SizeLimit("token information length is invalid"))?;
    if required == 0 || required > MAX_TOKEN_INFORMATION_BYTES {
        return Err(TransportError::SizeLimit(
            "token information exceeds bounded allocation",
        ));
    }
    let mut buffer = AlignedBuffer::new(required)?;
    let mut returned = 0u32;
    let success = unsafe {
        GetTokenInformation(
            token,
            class,
            buffer.as_mut_ptr(),
            u32::try_from(buffer.byte_len()).expect("bounded token buffer fits u32"),
            &mut returned,
        )
    };
    if success == 0 {
        return Err(last_error("GetTokenInformation"));
    }
    let returned = usize::try_from(returned)
        .map_err(|_| TransportError::SizeLimit("returned token length is invalid"))?;
    if returned == 0 || returned > buffer.byte_len() {
        return Err(TransportError::InvalidTokenIdentity(
            "returned token length is outside allocation",
        ));
    }
    buffer.set_byte_len(returned);
    Ok(buffer)
}

fn extract_logon_sid(buffer: &AlignedBuffer) -> Result<Vec<u8>, TransportError> {
    let header = mem::offset_of!(TOKEN_GROUPS, Groups);
    if buffer.byte_len() < header {
        return Err(TransportError::InvalidTokenIdentity(
            "token groups buffer is truncated",
        ));
    }
    let groups = unsafe { &*(buffer.as_ptr().cast::<TOKEN_GROUPS>()) };
    let count = usize::try_from(groups.GroupCount)
        .map_err(|_| TransportError::InvalidTokenIdentity("group count is invalid"))?;
    let available = buffer.byte_len() - header;
    let maximum = available / mem::size_of::<SID_AND_ATTRIBUTES>();
    if count > maximum {
        return Err(TransportError::InvalidTokenIdentity(
            "token group count exceeds buffer",
        ));
    }
    let first = unsafe { buffer.as_ptr().add(header).cast::<SID_AND_ATTRIBUTES>() };
    let entries = unsafe { slice::from_raw_parts(first, count) };
    let mut matched = None;
    for entry in entries {
        let attributes = entry.Attributes as i32;
        if attributes & SE_GROUP_LOGON_ID == SE_GROUP_LOGON_ID
            && attributes & SE_GROUP_ENABLED == SE_GROUP_ENABLED
        {
            if matched.is_some() {
                return Err(TransportError::InvalidTokenIdentity(
                    "multiple enabled logon SIDs found",
                ));
            }
            matched = Some(copy_sid(entry.Sid)?);
        }
    }
    matched.ok_or(TransportError::InvalidTokenIdentity(
        "enabled logon SID not found",
    ))
}

fn copy_sid(sid: *mut c_void) -> Result<Vec<u8>, TransportError> {
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(TransportError::InvalidTokenIdentity("SID is invalid"));
    }
    let length = usize::try_from(unsafe { GetLengthSid(sid) })
        .map_err(|_| TransportError::InvalidTokenIdentity("SID length is invalid"))?;
    if length == 0 || length > MAX_TOKEN_INFORMATION_BYTES {
        return Err(TransportError::InvalidTokenIdentity(
            "SID length is outside bounds",
        ));
    }
    Ok(unsafe { slice::from_raw_parts(sid.cast::<u8>(), length) }.to_vec())
}

fn sid_last_sub_authority(sid: *mut c_void) -> Result<u32, TransportError> {
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(TransportError::InvalidTokenIdentity(
            "integrity SID is invalid",
        ));
    }
    let count_pointer = unsafe { GetSidSubAuthorityCount(sid) };
    if count_pointer.is_null() {
        return Err(TransportError::InvalidTokenIdentity(
            "integrity SID count is unavailable",
        ));
    }
    let count = unsafe { *count_pointer };
    if count == 0 {
        return Err(TransportError::InvalidTokenIdentity(
            "integrity SID has no sub-authority",
        ));
    }
    let rid = unsafe { GetSidSubAuthority(sid, u32::from(count - 1)) };
    if rid.is_null() {
        return Err(TransportError::InvalidTokenIdentity(
            "integrity RID is unavailable",
        ));
    }
    Ok(unsafe { *rid })
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE, operation: &'static str) -> Result<Self, TransportError> {
        if handle.is_null() {
            return Err(last_error(operation));
        }
        Ok(Self(handle))
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct AlignedBuffer {
    words: Vec<usize>,
    byte_len: usize,
}

impl AlignedBuffer {
    fn new(byte_len: usize) -> Result<Self, TransportError> {
        let word = mem::size_of::<usize>();
        let words = byte_len
            .checked_add(word - 1)
            .ok_or(TransportError::SizeLimit("token buffer length overflow"))?
            / word;
        Ok(Self {
            words: vec![0; words],
            byte_len: words * word,
        })
    }

    fn as_ptr(&self) -> *const u8 {
        self.words.as_ptr().cast()
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.words.as_mut_ptr().cast()
    }

    fn byte_len(&self) -> usize {
        self.byte_len
    }

    fn set_byte_len(&mut self, value: usize) {
        self.byte_len = value;
    }
}

pub(crate) fn move_file_no_replace(
    source: &Path,
    destination: &Path,
) -> Result<bool, TransportError> {
    let source = wide_path(source)?;
    let destination = wide_path(destination)?;
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded != 0 {
        return Ok(true);
    }
    let code = unsafe { GetLastError() };
    if code == ERROR_ALREADY_EXISTS || code == ERROR_FILE_EXISTS {
        return Ok(false);
    }
    Err(TransportError::Windows {
        operation: "MoveFileExW",
        code,
    })
}

fn wide_path(path: &Path) -> Result<Vec<u16>, TransportError> {
    let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if value.contains(&0) {
        return Err(TransportError::InvalidPipeName(
            "filesystem path contains NUL",
        ));
    }
    value.push(0);
    Ok(value)
}

fn last_error(operation: &'static str) -> TransportError {
    TransportError::Windows {
        operation,
        code: unsafe { GetLastError() },
    }
}
