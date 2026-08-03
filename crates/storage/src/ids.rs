use uuid::Uuid;

use crate::StorageError;

pub(crate) fn uuid_from_blob(value: &[u8]) -> Result<Uuid, StorageError> {
    Uuid::from_slice(value).map_err(|error| StorageError::Domain(error.to_string()))
}

pub(crate) fn typed_id_from_blob<T>(
    value: &[u8],
    constructor: impl FnOnce(Uuid) -> Result<T, pastral_domain::DomainError>,
) -> Result<T, StorageError> {
    constructor(uuid_from_blob(value)?).map_err(|error| StorageError::Domain(error.to_string()))
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_and_stable() {
        assert_eq!(hex_lower(&[0x00, 0xab, 0xff]), "00abff");
    }
}
