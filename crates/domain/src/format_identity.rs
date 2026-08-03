use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StandardFormatId(u32);

impl StandardFormatId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegisteredFormatName(String);

impl RegisteredFormatName {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DomainError::RegisteredFormatNameEmpty);
        }
        if value.contains('\0') {
            return Err(DomainError::RegisteredFormatNameContainsNul);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClipboardFormatIdentity {
    Standard(StandardFormatId),
    Registered(RegisteredFormatName),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_registered_names_are_rejected() {
        assert_eq!(
            RegisteredFormatName::new(""),
            Err(DomainError::RegisteredFormatNameEmpty)
        );
        assert_eq!(
            RegisteredFormatName::new("a\0b"),
            Err(DomainError::RegisteredFormatNameContainsNul)
        );
    }

    #[test]
    fn registered_identity_contains_only_the_stable_name() {
        let name = RegisteredFormatName::new("HTML Format").unwrap();
        let identity = ClipboardFormatIdentity::Registered(name.clone());
        assert_eq!(name.as_str(), "HTML Format");
        assert_eq!(identity, ClipboardFormatIdentity::Registered(name));
    }
}
