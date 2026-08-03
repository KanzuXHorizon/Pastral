use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UtcUnixMicros(i64);

impl UtcUnixMicros {
    pub const MIN: i64 = -62_135_596_800_000_000;
    pub const MAX: i64 = 253_402_300_799_999_999;

    pub fn new(value: i64) -> Result<Self, DomainError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(DomainError::UtcMicrosOutOfRange);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_boundaries() {
        assert_eq!(
            UtcUnixMicros::new(UtcUnixMicros::MIN).unwrap().get(),
            UtcUnixMicros::MIN
        );
        assert_eq!(
            UtcUnixMicros::new(UtcUnixMicros::MAX).unwrap().get(),
            UtcUnixMicros::MAX
        );
    }

    #[test]
    fn rejects_values_outside_supported_range() {
        assert_eq!(
            UtcUnixMicros::new(UtcUnixMicros::MIN - 1),
            Err(DomainError::UtcMicrosOutOfRange)
        );
        assert_eq!(
            UtcUnixMicros::new(UtcUnixMicros::MAX + 1),
            Err(DomainError::UtcMicrosOutOfRange)
        );
    }
}
