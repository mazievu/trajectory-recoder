//! Explicit, file-backed runtime configuration for Windows capture clients.
//!
//! This intentionally does not read process environment variables. Services
//! and interactive launchers load the same `client.env` file and pass only the
//! validated client values to their child processes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const REQUIRED_KEYS: [&str; 5] = [
    "DEPLOYMENT_ROLE",
    "TRAJECTORY_SERVER_URL",
    "TRAJECTORY_MACHINE_ID",
    "TRAJECTORY_USER_ID",
    "SPOOL_DIR",
];
const OPTIONAL_KEYS: [&str; 3] = [
    "TRAJECTORY_ENROLLMENT_TOKEN",
    "DEVICE_TOKEN",
    "TRAJECTORY_DEVICE_TOKEN_PATH",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRuntimeConfig {
    pub server_url: String,
    pub machine_id: String,
    pub user_id: String,
    pub spool_dir: PathBuf,
    pub enrollment_token: Option<String>,
    pub device_token: Option<String>,
    pub device_token_path: Option<PathBuf>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientRuntimeConfigError {
    #[error("could not read client configuration: {0}")]
    Io(String),
    #[error("invalid client configuration at line {line}: {reason}")]
    InvalidLine { line: usize, reason: String },
    #[error("duplicate client configuration key: {0}")]
    DuplicateKey(String),
    #[error("unknown or server-only client configuration key: {0}")]
    UnknownKey(String),
    #[error("missing required client configuration key: {0}")]
    MissingKey(String),
    #[error("DEPLOYMENT_ROLE must be client")]
    WrongRole,
    #[error("TRAJECTORY_SERVER_URL must be an explicit non-loopback HTTPS endpoint")]
    UnsafeServerUrl,
}

impl ClientRuntimeConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ClientRuntimeConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| ClientRuntimeConfigError::Io(error.to_string()))?;
        let mut pairs = Vec::new();
        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) =
                trimmed
                    .split_once('=')
                    .ok_or_else(|| ClientRuntimeConfigError::InvalidLine {
                        line: index + 1,
                        reason: "expected KEY=VALUE".to_string(),
                    })?;
            if key.trim().is_empty() {
                return Err(ClientRuntimeConfigError::InvalidLine {
                    line: index + 1,
                    reason: "key cannot be empty".to_string(),
                });
            }
            pairs.push((key.trim().to_string(), value.trim().to_string()));
        }
        Self::from_owned_pairs(pairs)
    }

    pub fn from_pairs<I, K, V>(pairs: I) -> Result<Self, ClientRuntimeConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        Self::from_owned_pairs(
            pairs
                .into_iter()
                .map(|(key, value)| (key.as_ref().to_string(), value.as_ref().to_string())),
        )
    }

    fn from_owned_pairs<I>(pairs: I) -> Result<Self, ClientRuntimeConfigError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut values = BTreeMap::new();
        for (key, value) in pairs {
            if !REQUIRED_KEYS.contains(&key.as_str()) && !OPTIONAL_KEYS.contains(&key.as_str()) {
                return Err(ClientRuntimeConfigError::UnknownKey(key));
            }
            if values
                .insert(key.clone(), value.trim().to_string())
                .is_some()
            {
                return Err(ClientRuntimeConfigError::DuplicateKey(key));
            }
        }

        for key in REQUIRED_KEYS {
            if values.get(key).is_none_or(|value| value.is_empty()) {
                return Err(ClientRuntimeConfigError::MissingKey(key.to_string()));
            }
        }
        if values["DEPLOYMENT_ROLE"] != "client" {
            return Err(ClientRuntimeConfigError::WrongRole);
        }
        let server_url = values["TRAJECTORY_SERVER_URL"]
            .trim_end_matches('/')
            .to_string();
        if !is_safe_client_server_url(&server_url) {
            return Err(ClientRuntimeConfigError::UnsafeServerUrl);
        }

        Ok(Self {
            server_url,
            machine_id: values["TRAJECTORY_MACHINE_ID"].clone(),
            user_id: values["TRAJECTORY_USER_ID"].clone(),
            spool_dir: PathBuf::from(&values["SPOOL_DIR"]),
            enrollment_token: optional_value(&values, "TRAJECTORY_ENROLLMENT_TOKEN"),
            device_token: optional_value(&values, "DEVICE_TOKEN"),
            device_token_path: optional_value(&values, "TRAJECTORY_DEVICE_TOKEN_PATH")
                .map(PathBuf::from),
        })
    }

    /// Environment entries explicitly passed to the uploader child. The host
    /// service must use `env_clear()` first so server/machine-wide variables
    /// cannot alter a client's destination or identity.
    pub fn child_environment(&self) -> Vec<(&'static str, String)> {
        let mut values = vec![
            ("DEPLOYMENT_ROLE", "client".to_string()),
            ("TRAJECTORY_SERVER_URL", self.server_url.clone()),
            ("TRAJECTORY_MACHINE_ID", self.machine_id.clone()),
            ("TRAJECTORY_USER_ID", self.user_id.clone()),
            ("SPOOL_DIR", self.spool_dir.display().to_string()),
        ];
        if let Some(value) = &self.enrollment_token {
            values.push(("TRAJECTORY_ENROLLMENT_TOKEN", value.clone()));
        }
        if let Some(value) = &self.device_token {
            values.push(("DEVICE_TOKEN", value.clone()));
        }
        if let Some(value) = &self.device_token_path {
            values.push(("TRAJECTORY_DEVICE_TOKEN_PATH", value.display().to_string()));
        }
        values
    }
}

fn optional_value(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values.get(key).filter(|value| !value.is_empty()).cloned()
}

fn is_safe_client_server_url(value: &str) -> bool {
    let Some(remainder) = value.strip_prefix("https://") else {
        return false;
    };
    let host = remainder.split('/').next().unwrap_or_default();
    let host = host
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    !host.is_empty() && !matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1")
}

#[cfg(windows)]
pub fn default_client_config_path() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\TrajectoryRecorder\client.env")
}

#[cfg(not(windows))]
pub fn default_client_config_path() -> PathBuf {
    PathBuf::from("client.env")
}
