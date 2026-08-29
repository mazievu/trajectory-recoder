use crate::schema::RecorderConfig;
use crate::validation::Validate;
use arc_swap::ArcSwap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Validation error: {0}")]
    Validation(#[from] crate::validation::ConfigValidationError),
}

/// Thread-safe configuration container with lock-free atomic read access and live updates.
pub struct ConfigManager {
    current: ArcSwap<RecorderConfig>,
    config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// Creates a ConfigManager from an in-memory configuration.
    pub fn new(config: RecorderConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self {
            current: ArcSwap::from_pointee(config),
            config_path: None,
        })
    }

    /// Loads, parses, and validates configuration from a TOML file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path_buf = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path_buf)?;
        let config: RecorderConfig = toml::from_str(&content)?;
        config.validate()?;

        info!(path = %path_buf.display(), "Loaded and validated recorder configuration");
        Ok(Self {
            current: ArcSwap::from_pointee(config),
            config_path: Some(path_buf),
        })
    }

    /// Returns a shared, atomic snapshot of the current configuration.
    #[inline]
    pub fn get(&self) -> Arc<RecorderConfig> {
        self.current.load_full()
    }

    /// Atomically swaps the active configuration after full validation.
    pub fn update(&self, new_config: RecorderConfig) -> Result<(), ConfigError> {
        new_config.validate()?;
        self.current.store(Arc::new(new_config));
        info!("Recorder configuration updated successfully");
        Ok(())
    }

    /// Reloads configuration from the registered file path.
    pub fn reload(&self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.config_path {
            let content = fs::read_to_string(path)?;
            let new_config: RecorderConfig = toml::from_str(&content)?;
            self.update(new_config)?;
            info!(path = %path.display(), "Reloaded configuration from disk");
        } else {
            warn!("Cannot reload: no config file path registered");
        }
        Ok(())
    }

    /// Applies server policy overrides on top of the active configuration.
    pub fn apply_server_policy_override(
        &self,
        override_fn: impl FnOnce(&mut RecorderConfig),
    ) -> Result<(), ConfigError> {
        let mut cloned = (*self.get()).clone();
        override_fn(&mut cloned);
        self.update(cloned)
    }

    /// Saves the active configuration to disk in TOML format.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(&*self.get())?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_manager_atomic_update() {
        let manager = ConfigManager::new(RecorderConfig::default()).unwrap();
        assert_eq!(manager.get().capture.video_fps, 10);

        let mut new_config = (*manager.get()).clone();
        new_config.capture.video_fps = 15;
        manager.update(new_config).unwrap();

        assert_eq!(manager.get().capture.video_fps, 15);
    }

    #[test]
    fn test_server_policy_override() {
        let manager = ConfigManager::new(RecorderConfig::default()).unwrap();
        manager
            .apply_server_policy_override(|cfg| {
                cfg.privacy
                    .excluded_apps
                    .push("OverriddenApp.exe".to_string());
            })
            .unwrap();

        assert!(
            manager
                .get()
                .privacy
                .excluded_apps
                .contains(&"OverriddenApp.exe".to_string())
        );
    }
}
