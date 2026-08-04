#![cfg(windows)]

use std::{ffi::OsString, path::PathBuf};

use pastral_agent::{AgentRuntimeError, resolve_resident_data_root_from};

#[test]
fn explicit_resident_root_wins_without_environment_lookup() {
    let explicit = PathBuf::from(r"D:\PastralData");
    assert_eq!(
        resolve_resident_data_root_from(Some(explicit.clone()), None),
        Ok(explicit)
    );
}

#[test]
fn default_resident_root_is_local_app_data_pastral() {
    assert_eq!(
        resolve_resident_data_root_from(
            None,
            Some(OsString::from(r"C:\Users\Example\AppData\Local")),
        ),
        Ok(PathBuf::from(r"C:\Users\Example\AppData\Local\Pastral"))
    );
}

#[test]
fn missing_empty_relative_and_unc_roots_fail_closed() {
    assert_eq!(
        resolve_resident_data_root_from(None, None),
        Err(AgentRuntimeError::InvalidDataRoot)
    );
    assert_eq!(
        resolve_resident_data_root_from(None, Some(OsString::new())),
        Err(AgentRuntimeError::InvalidDataRoot)
    );
    assert_eq!(
        resolve_resident_data_root_from(Some(PathBuf::from("relative")), None),
        Err(AgentRuntimeError::InvalidDataRoot)
    );
    assert_eq!(
        resolve_resident_data_root_from(Some(PathBuf::from(r"\\server\share")), None),
        Err(AgentRuntimeError::InvalidDataRoot)
    );
}
