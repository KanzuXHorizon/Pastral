use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use pastral_agent_core::SourceAdmissionPolicy;
use pastral_domain::ProtectionDomainId;

use crate::AgentRuntimeError;

const PRIVACY_POLICY_FILE: &str = "privacy-policy.txt";
const PRIVACY_POLICY_VERSION: &str = "1";
const DEFAULT_DENIED_EXECUTABLES: [&str; 4] = [
    "1password.exe",
    "bitwarden.exe",
    "keepass.exe",
    "keepassxc.exe",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyPolicyConfig {
    source_policy: SourceAdmissionPolicy,
}

impl PrivacyPolicyConfig {
    pub fn load_or_create(root: &Path) -> Result<Self, AgentRuntimeError> {
        fs::create_dir_all(root)
            .map_err(|error| AgentRuntimeError::io("create privacy policy root", &error))?;
        let final_path = root.join(PRIVACY_POLICY_FILE);
        if final_path.is_file() {
            return Self::load(&final_path);
        }

        let config = Self::default_policy()?;
        let temporary_path = root.join(format!(
            ".privacy-policy-{}.tmp",
            ProtectionDomainId::new_v4()
        ));
        let content = config.encode();
        let write_result = (|| -> Result<(), AgentRuntimeError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|error| {
                    AgentRuntimeError::io("create privacy policy staging file", &error)
                })?;
            file.write_all(content.as_bytes()).map_err(|error| {
                AgentRuntimeError::io("write privacy policy staging file", &error)
            })?;
            file.sync_all().map_err(|error| {
                AgentRuntimeError::io("sync privacy policy staging file", &error)
            })?;
            match fs::rename(&temporary_path, &final_path) {
                Ok(()) => Ok(()),
                Err(_) if final_path.is_file() => Ok(()),
                Err(error) => Err(AgentRuntimeError::io("publish privacy policy file", &error)),
            }
        })();
        if write_result.is_err() || final_path.is_file() {
            let _ = fs::remove_file(&temporary_path);
        }
        write_result?;
        Self::load(&final_path)
    }

    fn default_policy() -> Result<Self, AgentRuntimeError> {
        SourceAdmissionPolicy::new(true, DEFAULT_DENIED_EXECUTABLES)
            .map(|source_policy| Self { source_policy })
            .map_err(|_| AgentRuntimeError::InvalidPrivacyPolicy("default source policy"))
    }

    fn load(path: &Path) -> Result<Self, AgentRuntimeError> {
        let content = fs::read_to_string(path)
            .map_err(|error| AgentRuntimeError::io("read privacy policy file", &error))?;
        let mut version = None;
        let mut deny_unresolved_source = None;
        let mut denied_executable_names = Vec::new();

        for line in content.lines() {
            if line.is_empty() {
                return Err(AgentRuntimeError::InvalidPrivacyPolicy("empty line"));
            }
            if let Some(value) = line.strip_prefix("version=") {
                if version.replace(value).is_some() {
                    return Err(AgentRuntimeError::InvalidPrivacyPolicy("duplicate version"));
                }
                continue;
            }
            if let Some(value) = line.strip_prefix("deny_unresolved_source=") {
                if deny_unresolved_source.is_some() {
                    return Err(AgentRuntimeError::InvalidPrivacyPolicy(
                        "duplicate unresolved-source policy",
                    ));
                }
                deny_unresolved_source = Some(match value {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(AgentRuntimeError::InvalidPrivacyPolicy(
                            "invalid unresolved-source policy",
                        ));
                    }
                });
                continue;
            }
            if let Some(value) = line.strip_prefix("deny_process=") {
                denied_executable_names.push(value.to_owned());
                continue;
            }
            return Err(AgentRuntimeError::InvalidPrivacyPolicy("unknown field"));
        }

        if version != Some(PRIVACY_POLICY_VERSION) {
            return Err(AgentRuntimeError::InvalidPrivacyPolicy(
                "unsupported or missing version",
            ));
        }
        let deny_unresolved_source = deny_unresolved_source.ok_or(
            AgentRuntimeError::InvalidPrivacyPolicy("missing unresolved-source policy"),
        )?;
        let source_policy =
            SourceAdmissionPolicy::new(deny_unresolved_source, denied_executable_names)
                .map_err(|_| AgentRuntimeError::InvalidPrivacyPolicy("invalid denied process"))?;
        Ok(Self { source_policy })
    }

    fn encode(&self) -> String {
        let mut content = format!(
            "version={PRIVACY_POLICY_VERSION}\ndeny_unresolved_source={}\n",
            self.source_policy.deny_unresolved_source()
        );
        for executable in self.source_policy.denied_executable_names() {
            content.push_str("deny_process=");
            content.push_str(executable);
            content.push('\n');
        }
        content
    }

    #[must_use]
    pub const fn source_policy(&self) -> &SourceAdmissionPolicy {
        &self.source_policy
    }
}
