use crate::ClipboardError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedUnicodeText {
    raw_logical_bytes: Vec<u8>,
    text: String,
}

impl CapturedUnicodeText {
    pub(crate) fn parse(allocation_bytes: &[u8]) -> Result<Self, ClipboardError> {
        let mut units = Vec::new();
        let mut logical_end = None;
        let mut offset = 0usize;
        while offset + 1 < allocation_bytes.len() {
            let unit = u16::from_le_bytes([allocation_bytes[offset], allocation_bytes[offset + 1]]);
            offset += 2;
            if unit == 0 {
                logical_end = Some(offset);
                break;
            }
            units.push(unit);
        }

        let logical_end = match logical_end {
            Some(value) => value,
            None if !allocation_bytes.len().is_multiple_of(2) => {
                return Err(ClipboardError::UnicodeTextMisaligned);
            }
            None => return Err(ClipboardError::UnicodeTextMissingTerminator),
        };
        let text =
            String::from_utf16(&units).map_err(|_| ClipboardError::UnicodeTextInvalidUtf16)?;
        Ok(Self {
            raw_logical_bytes: allocation_bytes[..logical_end].to_vec(),
            text,
        })
    }

    #[must_use]
    pub fn raw_logical_bytes(&self) -> &[u8] {
        &self.raw_logical_bytes
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(value: &str) -> Vec<u8> {
        value
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    #[test]
    fn empty_ascii_bmp_surrogate_and_crlf_are_preserved() {
        for value in ["", "plain", "Tiếng Việt", "😀", "line1\r\nline2"] {
            let bytes = encoded(value);
            let captured = CapturedUnicodeText::parse(&bytes).unwrap();
            assert_eq!(captured.raw_logical_bytes(), bytes);
            assert_eq!(captured.text(), value);
        }
    }

    #[test]
    fn first_terminator_defines_logical_bytes_and_excludes_padding() {
        let mut bytes = encoded("text");
        let expected = bytes.clone();
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let captured = CapturedUnicodeText::parse(&bytes).unwrap();
        assert_eq!(captured.raw_logical_bytes(), expected);
        assert_eq!(captured.text(), "text");
    }

    #[test]
    fn normalization_is_not_applied() {
        let precomposed = CapturedUnicodeText::parse(&encoded("é")).unwrap();
        let decomposed = CapturedUnicodeText::parse(&encoded("e\u{301}")).unwrap();
        assert_ne!(
            precomposed.raw_logical_bytes(),
            decomposed.raw_logical_bytes()
        );
        assert_ne!(precomposed.text(), decomposed.text());
    }

    #[test]
    fn missing_terminator_partial_unit_and_invalid_surrogate_are_rejected() {
        assert_eq!(
            CapturedUnicodeText::parse(&[b'a', 0]),
            Err(ClipboardError::UnicodeTextMissingTerminator)
        );
        assert_eq!(
            CapturedUnicodeText::parse(&[b'a', 0, b'b']),
            Err(ClipboardError::UnicodeTextMisaligned)
        );
        assert_eq!(
            CapturedUnicodeText::parse(&[0x00, 0xD8, 0x00, 0x00]),
            Err(ClipboardError::UnicodeTextInvalidUtf16)
        );
    }
}
