use crate::{TransportError, sys};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessMemorySnapshot {
    working_set_bytes: u64,
    private_usage_bytes: u64,
}

impl ProcessMemorySnapshot {
    pub(crate) const fn new(
        working_set_bytes: u64,
        private_usage_bytes: u64,
    ) -> Result<Self, TransportError> {
        if working_set_bytes == 0 {
            return Err(TransportError::InvalidProcessMemory(
                "working set must be nonzero",
            ));
        }
        if private_usage_bytes == 0 {
            return Err(TransportError::InvalidProcessMemory(
                "private usage must be nonzero",
            ));
        }
        Ok(Self {
            working_set_bytes,
            private_usage_bytes,
        })
    }

    #[must_use]
    pub const fn working_set_bytes(self) -> u64 {
        self.working_set_bytes
    }

    #[must_use]
    pub const fn private_usage_bytes(self) -> u64 {
        self.private_usage_bytes
    }
}

pub fn process_memory_snapshot(process_id: u32) -> Result<ProcessMemorySnapshot, TransportError> {
    if process_id == 0 {
        return Err(TransportError::InvalidProcessMemory(
            "process ID must be nonzero",
        ));
    }
    let raw = sys::query_process_memory(process_id)?;
    ProcessMemorySnapshot::new(raw.working_set_bytes, raw.private_usage_bytes)
}
