use crate::AgentError;

pub const MAX_SECRET_SCAN_BYTES: usize = 1024 * 1024;
const MAX_EXECUTABLE_NAME_BYTES: usize = 260;
const PRIVATE_KEY_MARKERS: [&str; 7] = [
    concat!("-----BEGIN ", "PRIVATE KEY-----"),
    concat!("-----BEGIN ENCRYPTED ", "PRIVATE KEY-----"),
    concat!("-----BEGIN RSA ", "PRIVATE KEY-----"),
    concat!("-----BEGIN EC ", "PRIVATE KEY-----"),
    concat!("-----BEGIN DSA ", "PRIVATE KEY-----"),
    concat!("-----BEGIN OPENSSH ", "PRIVATE KEY-----"),
    concat!("-----BEGIN PGP ", "PRIVATE KEY BLOCK-----"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceConfidence {
    Unavailable,
    ProcessImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObservation {
    executable_name: Option<String>,
    confidence: SourceConfidence,
}

impl SourceObservation {
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            executable_name: None,
            confidence: SourceConfidence::Unavailable,
        }
    }

    pub fn from_executable_name(value: impl AsRef<str>) -> Result<Self, AgentError> {
        let executable_name = normalize_executable_name(value.as_ref())?;
        Ok(Self {
            executable_name: Some(executable_name),
            confidence: SourceConfidence::ProcessImage,
        })
    }

    #[must_use]
    pub const fn confidence(&self) -> SourceConfidence {
        self.confidence
    }

    #[must_use]
    pub fn executable_name(&self) -> Option<&str> {
        self.executable_name.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAdmissionDecision {
    Allow,
    DenyUnresolved,
    DenyExecutable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdmissionPolicy {
    deny_unresolved_source: bool,
    denied_executable_names: Vec<String>,
}

impl SourceAdmissionPolicy {
    pub fn new<I, S>(deny_unresolved_source: bool, denied: I) -> Result<Self, AgentError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut denied_executable_names = denied
            .into_iter()
            .map(|value| normalize_executable_name(value.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        denied_executable_names.sort_unstable();
        denied_executable_names.dedup();
        Ok(Self {
            deny_unresolved_source,
            denied_executable_names,
        })
    }

    #[must_use]
    pub const fn deny_unresolved_source(&self) -> bool {
        self.deny_unresolved_source
    }

    #[must_use]
    pub fn denied_executable_names(&self) -> &[String] {
        &self.denied_executable_names
    }

    #[must_use]
    pub fn evaluate(&self, observation: &SourceObservation) -> SourceAdmissionDecision {
        let Some(name) = observation.executable_name() else {
            return if self.deny_unresolved_source {
                SourceAdmissionDecision::DenyUnresolved
            } else {
                SourceAdmissionDecision::Allow
            };
        };
        if self
            .denied_executable_names
            .binary_search_by(|candidate| candidate.as_str().cmp(name))
            .is_ok()
        {
            SourceAdmissionDecision::DenyExecutable
        } else {
            SourceAdmissionDecision::Allow
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveClass {
    PrivateKeyMaterial,
    DetectorLimitExceeded,
}

#[must_use]
pub fn detect_high_confidence_secret(text: &str) -> Option<SensitiveClass> {
    if text.len() > MAX_SECRET_SCAN_BYTES {
        return Some(SensitiveClass::DetectorLimitExceeded);
    }
    PRIVATE_KEY_MARKERS
        .iter()
        .any(|marker| text.contains(marker))
        .then_some(SensitiveClass::PrivateKeyMaterial)
}

fn normalize_executable_name(value: &str) -> Result<String, AgentError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_EXECUTABLE_NAME_BYTES
        || value.contains(['/', '\\', ':'])
    {
        return Err(AgentError::InvalidExecutableName);
    }
    Ok(value.to_lowercase())
}
