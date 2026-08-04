use crate::{TransportError, sys};

pub fn random_bytes<const N: usize>() -> Result<[u8; N], TransportError> {
    let mut bytes = [0u8; N];
    sys::fill_system_random(&mut bytes)?;
    Ok(bytes)
}
