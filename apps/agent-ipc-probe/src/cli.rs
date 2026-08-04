use std::{ffi::OsString, path::PathBuf};

use crate::AdmissionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionMode {
    Parent,
    BaselineChild { data_root: PathBuf },
    ServerChild { data_root: PathBuf },
}

pub fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<AdmissionMode, AdmissionError> {
    let values = arguments.into_iter().collect::<Vec<_>>();
    match values.as_slice() {
        [] => Ok(AdmissionMode::Parent),
        [mode, root_flag, root]
            if (mode == "--baseline-child" || mode == "--server-child")
                && root_flag == "--data-root"
                && !root.is_empty() =>
        {
            let data_root = PathBuf::from(root);
            if mode == "--baseline-child" {
                Ok(AdmissionMode::BaselineChild { data_root })
            } else {
                Ok(AdmissionMode::ServerChild { data_root })
            }
        }
        _ => Err(AdmissionError::InvalidArguments),
    }
}
