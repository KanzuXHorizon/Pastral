use core::num::NonZeroU32;

use pastral_domain::{ClipboardFormatIdentity, RegisteredFormatName, StandardFormatId};

use crate::ClipboardError;

pub const CF_UNICODETEXT_ID: u32 = 13;
pub(crate) const REGISTERED_FIRST: u32 = 0xC000;
pub(crate) const REGISTERED_LAST: u32 = 0xFFFF;
const PRIVATE_FIRST: u32 = 0x0200;
const PRIVATE_LAST: u32 = 0x02FF;
const GDI_OBJECT_FIRST: u32 = 0x0300;
const GDI_OBJECT_LAST: u32 = 0x03FF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeClipboardFormatId(NonZeroU32);

impl RuntimeClipboardFormatId {
    pub fn new(value: u32) -> Result<Self, ClipboardError> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ClipboardError::InvalidRuntimeFormatId)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeFormatKind {
    KnownStandard(ClipboardFormatIdentity),
    Registered(ClipboardFormatIdentity),
    Private,
    GdiObject,
    ReservedOrUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardFormatDescriptor {
    runtime_id: RuntimeClipboardFormatId,
    source_ordinal: usize,
    kind: RuntimeFormatKind,
}

impl ClipboardFormatDescriptor {
    #[must_use]
    pub const fn new(
        runtime_id: RuntimeClipboardFormatId,
        source_ordinal: usize,
        kind: RuntimeFormatKind,
    ) -> Self {
        Self {
            runtime_id,
            source_ordinal,
            kind,
        }
    }

    #[must_use]
    pub const fn runtime_id(&self) -> RuntimeClipboardFormatId {
        self.runtime_id
    }

    #[must_use]
    pub const fn source_ordinal(&self) -> usize {
        self.source_ordinal
    }

    #[must_use]
    pub const fn kind(&self) -> &RuntimeFormatKind {
        &self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeFormatClass {
    KnownStandard,
    Registered,
    Private,
    GdiObject,
    ReservedOrUnknown,
}

pub(crate) fn classify_runtime_id(id: RuntimeClipboardFormatId) -> RuntimeFormatClass {
    let value = id.get();
    if is_known_standard(value) {
        RuntimeFormatClass::KnownStandard
    } else if (PRIVATE_FIRST..=PRIVATE_LAST).contains(&value) {
        RuntimeFormatClass::Private
    } else if (GDI_OBJECT_FIRST..=GDI_OBJECT_LAST).contains(&value) {
        RuntimeFormatClass::GdiObject
    } else if (REGISTERED_FIRST..=REGISTERED_LAST).contains(&value) {
        RuntimeFormatClass::Registered
    } else {
        RuntimeFormatClass::ReservedOrUnknown
    }
}

pub(crate) fn known_standard_identity(
    id: RuntimeClipboardFormatId,
) -> Option<ClipboardFormatIdentity> {
    is_known_standard(id.get())
        .then(|| ClipboardFormatIdentity::Standard(StandardFormatId::new(id.get())))
}

pub(crate) fn registered_identity(name: String) -> Result<ClipboardFormatIdentity, ClipboardError> {
    RegisteredFormatName::new(name)
        .map(ClipboardFormatIdentity::Registered)
        .map_err(|_| ClipboardError::RegisteredNameInvalid)
}

const fn is_known_standard(value: u32) -> bool {
    matches!(
        value,
        1 | 2
            | 3
            | 4
            | 5
            | 6
            | 7
            | 8
            | 9
            | 10
            | 11
            | 12
            | 13
            | 14
            | 15
            | 16
            | 17
            | 0x0080
            | 0x0081
            | 0x0082
            | 0x0083
            | 0x008E
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_runtime_id_is_rejected() {
        assert_eq!(
            RuntimeClipboardFormatId::new(0),
            Err(ClipboardError::InvalidRuntimeFormatId)
        );
    }

    #[test]
    fn runtime_ranges_are_classified_without_durable_confusion() {
        assert_eq!(
            classify_runtime_id(RuntimeClipboardFormatId::new(CF_UNICODETEXT_ID).unwrap()),
            RuntimeFormatClass::KnownStandard
        );
        assert_eq!(
            classify_runtime_id(RuntimeClipboardFormatId::new(PRIVATE_FIRST).unwrap()),
            RuntimeFormatClass::Private
        );
        assert_eq!(
            classify_runtime_id(RuntimeClipboardFormatId::new(GDI_OBJECT_FIRST).unwrap()),
            RuntimeFormatClass::GdiObject
        );
        assert_eq!(
            classify_runtime_id(RuntimeClipboardFormatId::new(REGISTERED_FIRST).unwrap()),
            RuntimeFormatClass::Registered
        );
        assert_eq!(
            classify_runtime_id(RuntimeClipboardFormatId::new(0x1000).unwrap()),
            RuntimeFormatClass::ReservedOrUnknown
        );
    }

    #[test]
    fn registered_durable_identity_contains_name_not_runtime_id() {
        let identity = registered_identity("HTML Format".to_owned()).unwrap();
        assert_eq!(
            identity,
            ClipboardFormatIdentity::Registered(RegisteredFormatName::new("HTML Format").unwrap())
        );
    }
}
