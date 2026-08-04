use std::path::{Path, PathBuf};

use crate::{ClipboardError, sys};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardOwnerObservation {
    Unavailable,
    ProcessImage(PathBuf),
}

impl ClipboardOwnerObservation {
    pub(crate) fn observe() -> Result<Self, ClipboardError> {
        Ok(match sys::clipboard_owner_process_image()? {
            Some(path) => Self::ProcessImage(path),
            None => Self::Unavailable,
        })
    }

    #[must_use]
    pub fn process_image(&self) -> Option<&Path> {
        match self {
            Self::Unavailable => None,
            Self::ProcessImage(path) => Some(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_observation_exposes_no_path() {
        assert_eq!(ClipboardOwnerObservation::Unavailable.process_image(), None);
    }

    #[test]
    fn process_image_observation_preserves_path() {
        let observation = ClipboardOwnerObservation::ProcessImage(PathBuf::from("C:\\App.exe"));
        assert_eq!(observation.process_image(), Some(Path::new("C:\\App.exe")));
    }
}
