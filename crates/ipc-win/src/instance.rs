use crate::{TransportError, sys};

const LOCAL_PREFIX: &str = "Local\\";
const MAX_INSTANCE_NAME_UTF16: usize = 128;

pub struct LocalProcessInstanceGuard {
    _handle: sys::OwnedInstanceHandle,
}

pub enum LocalProcessInstance {
    Acquired(LocalProcessInstanceGuard),
    AlreadyRunning,
}

pub fn acquire_local_process_instance(name: &str) -> Result<LocalProcessInstance, TransportError> {
    let suffix = name
        .strip_prefix(LOCAL_PREFIX)
        .ok_or(TransportError::InvalidInstanceName(
            "name must use the Local namespace",
        ))?;
    if suffix.is_empty() {
        return Err(TransportError::InvalidInstanceName(
            "local instance name is empty",
        ));
    }
    if suffix.contains(['\\', '/']) {
        return Err(TransportError::InvalidInstanceName(
            "instance name contains a namespace separator",
        ));
    }
    if name.contains('\0') {
        return Err(TransportError::InvalidInstanceName(
            "instance name contains NUL",
        ));
    }

    let mut wide = name.encode_utf16().collect::<Vec<_>>();
    if wide.len() > MAX_INSTANCE_NAME_UTF16 {
        return Err(TransportError::InvalidInstanceName(
            "instance name exceeds the UTF-16 limit",
        ));
    }
    wide.push(0);

    let (handle, already_running) = sys::create_local_process_instance(&wide)?;
    if already_running {
        drop(handle);
        Ok(LocalProcessInstance::AlreadyRunning)
    } else {
        Ok(LocalProcessInstance::Acquired(LocalProcessInstanceGuard {
            _handle: handle,
        }))
    }
}
