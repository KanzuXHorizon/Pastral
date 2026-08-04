use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use pastral_domain::{ProfileId, ProtectionDomain, ProtectionDomainId};

use crate::AgentRuntimeError;

const IDENTITY_FILE: &str = "agent-identity.txt";
const IDENTITY_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentIdentity {
    profile_id: ProfileId,
    ordinary_domain_id: ProtectionDomainId,
}

impl AgentIdentity {
    pub fn load_or_create(root: &Path) -> Result<Self, AgentRuntimeError> {
        fs::create_dir_all(root)
            .map_err(|error| AgentRuntimeError::io("create identity root", &error))?;
        let final_path = root.join(IDENTITY_FILE);
        if final_path.is_file() {
            return Self::load(&final_path);
        }

        let identity = Self {
            profile_id: ProfileId::new_v4(),
            ordinary_domain_id: ProtectionDomainId::new_v4(),
        };
        let temporary_path = root.join(format!(
            ".agent-identity-{}.tmp",
            ProtectionDomainId::new_v4()
        ));
        let content = identity.encode();
        let write_result = (|| -> Result<(), AgentRuntimeError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|error| AgentRuntimeError::io("create identity staging file", &error))?;
            file.write_all(content.as_bytes())
                .map_err(|error| AgentRuntimeError::io("write identity staging file", &error))?;
            file.sync_all()
                .map_err(|error| AgentRuntimeError::io("sync identity staging file", &error))?;
            match fs::rename(&temporary_path, &final_path) {
                Ok(()) => Ok(()),
                Err(_) if final_path.is_file() => Ok(()),
                Err(error) => Err(AgentRuntimeError::io("publish identity file", &error)),
            }
        })();
        if write_result.is_err() || final_path.is_file() {
            let _ = fs::remove_file(&temporary_path);
        }
        write_result?;
        Self::load(&final_path)
    }

    fn load(path: &Path) -> Result<Self, AgentRuntimeError> {
        let content = fs::read_to_string(path)
            .map_err(|error| AgentRuntimeError::io("read identity file", &error))?;
        let lines = content.lines().collect::<Vec<_>>();
        if lines.len() != 3 {
            return Err(AgentRuntimeError::InvalidIdentity("unexpected line count"));
        }
        let version = lines[0]
            .strip_prefix("version=")
            .ok_or(AgentRuntimeError::InvalidIdentity("missing version"))?;
        if version != IDENTITY_VERSION {
            return Err(AgentRuntimeError::InvalidIdentity("unsupported version"));
        }
        let profile_id = lines[1]
            .strip_prefix("profile_id=")
            .ok_or(AgentRuntimeError::InvalidIdentity("missing profile ID"))
            .and_then(|value| {
                ProfileId::parse_str(value)
                    .map_err(|_| AgentRuntimeError::InvalidIdentity("invalid profile ID"))
            })?;
        let ordinary_domain_id = lines[2]
            .strip_prefix("ordinary_domain_id=")
            .ok_or(AgentRuntimeError::InvalidIdentity(
                "missing ordinary domain ID",
            ))
            .and_then(|value| {
                ProtectionDomainId::parse_str(value)
                    .map_err(|_| AgentRuntimeError::InvalidIdentity("invalid ordinary domain ID"))
            })?;
        Ok(Self {
            profile_id,
            ordinary_domain_id,
        })
    }

    fn encode(self) -> String {
        format!(
            "version={IDENTITY_VERSION}\nprofile_id={}\nordinary_domain_id={}\n",
            self.profile_id, self.ordinary_domain_id
        )
    }

    #[must_use]
    pub const fn profile_id(self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn ordinary_domain_id(self) -> ProtectionDomainId {
        self.ordinary_domain_id
    }

    #[must_use]
    pub const fn protection_domain(self) -> ProtectionDomain {
        ProtectionDomain::Ordinary(self.ordinary_domain_id)
    }
}
