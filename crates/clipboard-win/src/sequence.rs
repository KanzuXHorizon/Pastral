#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipboardSequence {
    Unavailable,
    Available(u32),
}

impl ClipboardSequence {
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        if value == 0 {
            Self::Unavailable
        } else {
            Self::Available(value)
        }
    }

    #[must_use]
    pub const fn raw(self) -> Option<u32> {
        match self {
            Self::Unavailable => None,
            Self::Available(value) => Some(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipboardNotification {
    sequence: ClipboardSequence,
}

impl ClipboardNotification {
    #[must_use]
    pub const fn new(sequence: ClipboardSequence) -> Self {
        Self { sequence }
    }

    #[must_use]
    pub const fn sequence(self) -> ClipboardSequence {
        self.sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_unavailable_and_nonzero_is_opaque() {
        assert_eq!(
            ClipboardSequence::from_raw(0),
            ClipboardSequence::Unavailable
        );
        assert_eq!(
            ClipboardSequence::from_raw(u32::MAX),
            ClipboardSequence::Available(u32::MAX)
        );
        assert_eq!(ClipboardSequence::Available(1).raw(), Some(1));
    }
}
