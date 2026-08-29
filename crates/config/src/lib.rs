//! Configuration schemas, default settings, validation rules, and live reload manager.

pub mod defaults;
pub mod manager;
pub mod schema;
pub mod validation;
pub mod watcher;

pub use defaults::default_recorder_config;
pub use manager::{ConfigError, ConfigManager};
pub use schema::{
    CaptureConfig, DiagnosticsConfig, MachineIdentityConfig, PrivacyConfig, RecorderConfig,
    ServerConfig, SpoolConfig, UploadConfig,
};
pub use validation::{ConfigValidationError, Validate};
pub use watcher::ConfigFileWatcher;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_generation() {
        let cfg = default_recorder_config();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.capture.video_fps, 10);
        assert_eq!(cfg.upload.chunk_size_mb, 64);
        assert_eq!(cfg.spool.local_retention_hours, 72);
    }
}
