use crate::{TokenIdentity, TransportError, sys};

pub struct PipeSecurity {
    native: sys::OwnedSecurityDescriptor,
    expected_logon_sid: Vec<u8>,
}

impl PipeSecurity {
    pub(crate) fn native(&self) -> &sys::OwnedSecurityDescriptor {
        &self.native
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityInspection {
    dacl_present: bool,
    dacl_defaulted: bool,
    dacl_protected: bool,
    ace_count: u32,
    allow_ace_count: u32,
    exact_logon_sid_match: bool,
    access_mask: u32,
}

impl SecurityInspection {
    #[must_use]
    pub const fn dacl_present(self) -> bool {
        self.dacl_present
    }
    #[must_use]
    pub const fn dacl_defaulted(self) -> bool {
        self.dacl_defaulted
    }
    #[must_use]
    pub const fn dacl_protected(self) -> bool {
        self.dacl_protected
    }
    #[must_use]
    pub const fn ace_count(self) -> u32 {
        self.ace_count
    }
    #[must_use]
    pub const fn allow_ace_count(self) -> u32 {
        self.allow_ace_count
    }
    #[must_use]
    pub const fn exact_logon_sid_match(self) -> bool {
        self.exact_logon_sid_match
    }
    #[must_use]
    pub const fn access_mask(self) -> u32 {
        self.access_mask
    }
}

pub fn build_logon_sid_pipe_security(
    identity: &TokenIdentity,
) -> Result<PipeSecurity, TransportError> {
    let native = sys::build_logon_sid_security_descriptor(identity.logon_sid().as_bytes())?;
    Ok(PipeSecurity {
        native,
        expected_logon_sid: identity.logon_sid().as_bytes().to_vec(),
    })
}

pub fn inspect_pipe_security(
    security: &PipeSecurity,
) -> Result<SecurityInspection, TransportError> {
    let raw = sys::inspect_security_descriptor(&security.native, &security.expected_logon_sid)?;
    Ok(SecurityInspection {
        dacl_present: raw.dacl_present,
        dacl_defaulted: raw.dacl_defaulted,
        dacl_protected: raw.dacl_protected,
        ace_count: raw.ace_count,
        allow_ace_count: raw.allow_ace_count,
        exact_logon_sid_match: raw.exact_logon_sid_match,
        access_mask: raw.access_mask,
    })
}
