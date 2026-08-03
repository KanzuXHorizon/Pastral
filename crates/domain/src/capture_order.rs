use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureOrder(u64);

impl CaptureOrder {
    pub fn new(value: u64) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::CaptureOrderMustBePositive);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_rejected() {
        assert_eq!(
            CaptureOrder::new(0),
            Err(DomainError::CaptureOrderMustBePositive)
        );
    }

    #[test]
    fn ordering_is_deterministic() {
        let mut values = [
            CaptureOrder::new(3).unwrap(),
            CaptureOrder::new(1).unwrap(),
            CaptureOrder::new(2).unwrap(),
        ];
        values.sort();
        assert_eq!(values.map(CaptureOrder::get), [1, 2, 3]);
    }
}
