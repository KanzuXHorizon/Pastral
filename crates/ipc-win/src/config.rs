use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use pastral_ipc_auth::{AUTH_MATERIAL_BYTES, InstallationSecret};
use pastral_ipc_core::CorrelationId;
use uuid::{Uuid, Variant, Version};

use crate::{
    MAX_SECRET_ENVELOPE_BYTES, TransportError, protect_installation_secret, random_bytes, sys,
    unprotect_installation_secret,
};

pub const IDENTITY_FILE_NAME: &str = "ipc-transport-identity.txt";
pub const SECRET_FILE_NAME: &str = "ipc-installation-secret.dpapi";
const IDENTITY_VERSION: u32 = 1;
const SECRET_VERSION: u32 = 1;
const MAX_IDENTITY_BYTES: u64 = 512;
const MAX_PIPE_NAME_UTF16_UNITS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportIdentity {
    instance_id: CorrelationId,
    secret_version: u32,
}

impl TransportIdentity {
    pub fn load_or_create(root: &Path) -> Result<Self, TransportError> {
        fs::create_dir_all(root)
            .map_err(|error| TransportError::io("create IPC transport root", &error))?;
        let final_path = root.join(IDENTITY_FILE_NAME);
        if final_path.is_file() {
            return Self::load(&final_path);
        }

        let identity = Self {
            instance_id: generate_instance_id()?,
            secret_version: SECRET_VERSION,
        };
        publish_new_file(
            root,
            &final_path,
            "ipc-identity",
            identity.encode().as_bytes(),
        )?;
        Self::load(&final_path)
    }

    fn load(path: &Path) -> Result<Self, TransportError> {
        let metadata = fs::metadata(path)
            .map_err(|error| TransportError::io("read IPC identity metadata", &error))?;
        if metadata.len() == 0 || metadata.len() > MAX_IDENTITY_BYTES {
            return Err(TransportError::InvalidIdentity(
                "identity length is outside bounds",
            ));
        }
        let content = fs::read_to_string(path)
            .map_err(|error| TransportError::io("read IPC identity", &error))?;
        let lines = content.lines().collect::<Vec<_>>();
        if lines.len() != 3 {
            return Err(TransportError::InvalidIdentity("unexpected line count"));
        }
        let version = parse_u32_line(lines[0], "version=")?;
        if version != IDENTITY_VERSION {
            return Err(TransportError::InvalidIdentity("unsupported version"));
        }
        let instance_text = lines[1]
            .strip_prefix("instance_id=")
            .ok_or(TransportError::InvalidIdentity("missing instance ID"))?;
        let uuid = Uuid::parse_str(instance_text)
            .map_err(|_| TransportError::InvalidIdentity("invalid instance ID"))?;
        if uuid.get_version() != Some(Version::Random) || uuid.get_variant() != Variant::RFC4122 {
            return Err(TransportError::InvalidIdentity(
                "instance ID must be UUIDv4 RFC4122",
            ));
        }
        if uuid.hyphenated().to_string() != instance_text {
            return Err(TransportError::InvalidIdentity(
                "instance ID is not canonical lowercase",
            ));
        }
        let instance_id = CorrelationId::from_bytes(*uuid.as_bytes())
            .map_err(|_| TransportError::InvalidIdentity("invalid instance ID"))?;
        let secret_version = parse_u32_line(lines[2], "secret_version=")?;
        if secret_version != SECRET_VERSION {
            return Err(TransportError::InvalidIdentity(
                "unsupported secret version",
            ));
        }

        let identity = Self {
            instance_id,
            secret_version,
        };
        if identity.encode() != content {
            return Err(TransportError::InvalidIdentity(
                "identity encoding is not canonical",
            ));
        }
        Ok(identity)
    }

    fn encode(self) -> String {
        let uuid = Uuid::from_bytes(*self.instance_id.as_bytes());
        format!(
            "version={IDENTITY_VERSION}\ninstance_id={}\nsecret_version={}\n",
            uuid.hyphenated(),
            self.secret_version
        )
    }

    #[must_use]
    pub const fn instance_id(self) -> CorrelationId {
        self.instance_id
    }

    #[must_use]
    pub const fn secret_version(self) -> u32 {
        self.secret_version
    }
}

pub struct TransportMaterial {
    identity: TransportIdentity,
    secret: InstallationSecret,
}

impl TransportMaterial {
    #[must_use]
    pub const fn identity(&self) -> &TransportIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn secret(&self) -> &InstallationSecret {
        &self.secret
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PipeName {
    text: String,
    wide_nul: Vec<u16>,
}

impl PipeName {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn as_wide_nul(&self) -> &[u16] {
        &self.wide_nul
    }
}

pub fn derive_pipe_name(
    identity: &TransportIdentity,
    session_id: u32,
) -> Result<PipeName, TransportError> {
    let uuid = Uuid::from_bytes(*identity.instance_id.as_bytes());
    let text = format!(r"\\.\pipe\Pastral-v1-s{session_id}-{}", uuid.hyphenated());
    if text.contains('\0') {
        return Err(TransportError::InvalidPipeName("pipe name contains NUL"));
    }
    let mut wide_nul = text.encode_utf16().collect::<Vec<_>>();
    if wide_nul.len() > MAX_PIPE_NAME_UTF16_UNITS {
        return Err(TransportError::InvalidPipeName(
            "pipe name exceeds UTF-16 bound",
        ));
    }
    wide_nul.push(0);
    Ok(PipeName { text, wide_nul })
}

pub fn load_or_create_transport_material(root: &Path) -> Result<TransportMaterial, TransportError> {
    let identity = TransportIdentity::load_or_create(root)?;
    let secret_path = root.join(SECRET_FILE_NAME);
    if !secret_path.is_file() {
        let secret = generate_installation_secret()?;
        let envelope = protect_installation_secret(&secret)?;
        publish_new_file(root, &secret_path, "ipc-secret", &envelope)?;
    }
    let envelope = read_bounded_file(
        &secret_path,
        MAX_SECRET_ENVELOPE_BYTES as u64,
        "read IPC secret metadata",
        "read IPC secret",
    )?;
    let secret = unprotect_installation_secret(&envelope)?;
    Ok(TransportMaterial { identity, secret })
}

fn generate_instance_id() -> Result<CorrelationId, TransportError> {
    let mut bytes = random_bytes::<16>()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    CorrelationId::from_bytes(bytes)
        .map_err(|_| TransportError::InvalidIdentity("generated instance ID is invalid"))
}

fn generate_installation_secret() -> Result<InstallationSecret, TransportError> {
    for _ in 0..4 {
        let bytes = random_bytes::<AUTH_MATERIAL_BYTES>()?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Ok(InstallationSecret::from_bytes(bytes));
        }
    }
    Err(TransportError::Windows {
        operation: "BCryptGenRandom returned all-zero secret repeatedly",
        code: 0,
    })
}

fn parse_u32_line(line: &str, prefix: &'static str) -> Result<u32, TransportError> {
    line.strip_prefix(prefix)
        .ok_or(TransportError::InvalidIdentity("missing required field"))?
        .parse::<u32>()
        .map_err(|_| TransportError::InvalidIdentity("integer field is invalid"))
}

fn publish_new_file(
    root: &Path,
    final_path: &Path,
    staging_label: &str,
    bytes: &[u8],
) -> Result<(), TransportError> {
    let staging_path = staging_path(root, staging_label);
    let result = (|| -> Result<(), TransportError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging_path)
            .map_err(|error| TransportError::io("create IPC staging file", &error))?;
        file.write_all(bytes)
            .map_err(|error| TransportError::io("write IPC staging file", &error))?;
        file.sync_all()
            .map_err(|error| TransportError::io("sync IPC staging file", &error))?;
        match sys::move_file_no_replace(&staging_path, final_path)? {
            true => Ok(()),
            false if final_path.is_file() => Ok(()),
            false => Err(TransportError::Windows {
                operation: "publish IPC material destination disappeared",
                code: 0,
            }),
        }
    })();
    if result.is_err() || final_path.is_file() {
        let _ = fs::remove_file(&staging_path);
    }
    result
}

fn staging_path(root: &Path, label: &str) -> PathBuf {
    root.join(format!(".{label}-{}.tmp", Uuid::new_v4()))
}

fn read_bounded_file(
    path: &Path,
    maximum: u64,
    metadata_operation: &'static str,
    read_operation: &'static str,
) -> Result<Vec<u8>, TransportError> {
    let metadata =
        fs::metadata(path).map_err(|error| TransportError::io(metadata_operation, &error))?;
    if metadata.len() == 0 || metadata.len() > maximum {
        return Err(TransportError::InvalidSecretEnvelope(
            "secret file length is outside bounds",
        ));
    }
    fs::read(path).map_err(|error| TransportError::io(read_operation, &error))
}
