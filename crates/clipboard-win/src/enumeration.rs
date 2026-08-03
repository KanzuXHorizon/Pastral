use core::num::NonZeroUsize;

use crate::{
    ClipboardError, ClipboardFormatDescriptor, RuntimeClipboardFormatId, RuntimeFormatKind,
    format::{
        RuntimeFormatClass, classify_runtime_id, known_standard_identity, registered_identity,
    },
    sys,
};

pub(crate) trait FormatSource {
    fn next_format(&mut self, previous: u32) -> Result<Option<u32>, ClipboardError>;
    fn registered_name(&mut self, format: u32) -> Result<String, ClipboardError>;
}

pub(crate) struct Win32FormatSource;

impl FormatSource for Win32FormatSource {
    fn next_format(&mut self, previous: u32) -> Result<Option<u32>, ClipboardError> {
        sys::enumerate_next(previous)
    }

    fn registered_name(&mut self, format: u32) -> Result<String, ClipboardError> {
        sys::registered_format_name(format)
    }
}

pub(crate) fn enumerate_formats(
    source: &mut impl FormatSource,
    max_formats: NonZeroUsize,
) -> Result<Vec<ClipboardFormatDescriptor>, ClipboardError> {
    let mut formats = Vec::new();
    let mut previous = 0u32;
    loop {
        let Some(raw) = source.next_format(previous)? else {
            return Ok(formats);
        };
        if formats.len() >= max_formats.get() {
            return Err(ClipboardError::FormatLimitExceeded {
                limit: max_formats.get(),
            });
        }
        let runtime_id = RuntimeClipboardFormatId::new(raw)?;
        let kind = match classify_runtime_id(runtime_id) {
            RuntimeFormatClass::KnownStandard => RuntimeFormatKind::KnownStandard(
                known_standard_identity(runtime_id)
                    .ok_or(ClipboardError::InvalidRuntimeFormatId)?,
            ),
            RuntimeFormatClass::Registered => {
                RuntimeFormatKind::Registered(registered_identity(source.registered_name(raw)?)?)
            }
            RuntimeFormatClass::Private => RuntimeFormatKind::Private,
            RuntimeFormatClass::GdiObject => RuntimeFormatKind::GdiObject,
            RuntimeFormatClass::ReservedOrUnknown => RuntimeFormatKind::ReservedOrUnknown,
        };
        let source_ordinal = formats.len();
        formats.push(ClipboardFormatDescriptor::new(
            runtime_id,
            source_ordinal,
            kind,
        ));
        previous = raw;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use pastral_domain::{ClipboardFormatIdentity, RegisteredFormatName, StandardFormatId};

    use super::*;

    struct FakeSource {
        values: VecDeque<Result<Option<u32>, ClipboardError>>,
        names: HashMap<u32, String>,
    }

    impl FormatSource for FakeSource {
        fn next_format(&mut self, _previous: u32) -> Result<Option<u32>, ClipboardError> {
            self.values.pop_front().unwrap_or(Ok(None))
        }

        fn registered_name(&mut self, format: u32) -> Result<String, ClipboardError> {
            self.names
                .get(&format)
                .cloned()
                .ok_or(ClipboardError::RegisteredNameInvalid)
        }
    }

    #[test]
    fn enumeration_preserves_source_order_and_resolves_stable_identity() {
        let registered = 0xC123;
        let mut source = FakeSource {
            values: VecDeque::from([
                Ok(Some(13)),
                Ok(Some(registered)),
                Ok(Some(0x0200)),
                Ok(None),
            ]),
            names: HashMap::from([(registered, "HTML Format".to_owned())]),
        };
        let formats = enumerate_formats(&mut source, NonZeroUsize::new(8).unwrap()).unwrap();
        assert_eq!(formats.len(), 3);
        assert_eq!(formats[0].source_ordinal(), 0);
        assert_eq!(formats[1].source_ordinal(), 1);
        assert_eq!(
            formats[0].kind(),
            &RuntimeFormatKind::KnownStandard(ClipboardFormatIdentity::Standard(
                StandardFormatId::new(13)
            ))
        );
        assert_eq!(
            formats[1].kind(),
            &RuntimeFormatKind::Registered(ClipboardFormatIdentity::Registered(
                RegisteredFormatName::new("HTML Format").unwrap()
            ))
        );
        assert_eq!(formats[2].kind(), &RuntimeFormatKind::Private);
    }

    #[test]
    fn count_bound_fails_before_growing_unbounded() {
        let mut source = FakeSource {
            values: VecDeque::from([Ok(Some(1)), Ok(Some(2)), Ok(None)]),
            names: HashMap::new(),
        };
        assert_eq!(
            enumerate_formats(&mut source, NonZeroUsize::new(1).unwrap()),
            Err(ClipboardError::FormatLimitExceeded { limit: 1 })
        );
    }

    #[test]
    fn source_error_is_not_misread_as_normal_completion() {
        let error = ClipboardError::win32("EnumClipboardFormats", 5);
        let mut source = FakeSource {
            values: VecDeque::from([Err(error.clone())]),
            names: HashMap::new(),
        };
        assert_eq!(
            enumerate_formats(&mut source, NonZeroUsize::new(4).unwrap()),
            Err(error)
        );
    }
}
