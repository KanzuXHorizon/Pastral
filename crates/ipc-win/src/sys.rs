use core::{ptr, slice};
use std::{os::windows::ffi::OsStrExt, path::Path};

use windows_sys::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, GetLastError, HLOCAL, LocalFree},
    Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom, CRYPT_INTEGER_BLOB,
        CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
    Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW},
};
use zeroize::Zeroizing;

use crate::TransportError;

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
