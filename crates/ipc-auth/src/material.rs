use zeroize::Zeroizing;

use crate::AuthError;

pub const AUTH_MATERIAL_BYTES: usize = 32;

pub struct InstallationSecret(Zeroizing<[u8; AUTH_MATERIAL_BYTES]>);

impl InstallationSecret {
    #[must_use]
    pub fn from_bytes(bytes: [u8; AUTH_MATERIAL_BYTES]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub fn expose<R>(&self, operation: impl FnOnce(&[u8; AUTH_MATERIAL_BYTES]) -> R) -> R {
        operation(&self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nonce([u8; AUTH_MATERIAL_BYTES]);

impl Nonce {
    pub fn from_bytes(bytes: [u8; AUTH_MATERIAL_BYTES]) -> Result<Self, AuthError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(AuthError::InvalidNonce);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; AUTH_MATERIAL_BYTES] {
        &self.0
    }
}

pub struct AuthenticationProof(Zeroizing<[u8; AUTH_MATERIAL_BYTES]>);

impl AuthenticationProof {
    #[must_use]
    pub fn from_bytes(bytes: [u8; AUTH_MATERIAL_BYTES]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; AUTH_MATERIAL_BYTES] {
        &self.0
    }
}
