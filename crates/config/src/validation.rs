use crate::schema::RecorderConfig;
use regex::Regex;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ConfigValidationError {
    #[error("Invalid version: {0}, must be >= 1")]
    InvalidVersion(u32),

    #[error("Invalid chunk size: {0} MiB, must be between 64 and 256 MiB")]
    InvalidChunkSize(usize),

    #[error("Invalid screenshot quality: {0}, must be between 1 and 100")]
    InvalidScreenshotQuality(u8),

    #[error("Invalid screenshot diff threshold: {0}, must be between 0.0 and 1.0")]
    InvalidDiffThreshold(f32),

    #[error("Invalid video FPS: {0}, must be between 1 and 60")]
    InvalidVideoFps(u32),

    #[error("Invalid video bitrate: {0} kbps, must be between 100 and 50000 kbps")]
    InvalidVideoBitrate(u32),

    #[error("Invalid entropy threshold: {0}, must be between 0.0 and 8.0")]
    InvalidEntropyThreshold(f64),

    #[error("Invalid disk pressure watermark ordering: L1({l1}%) must be < L2({l2}%) < L3({l3}%) <= 100%")]
    InvalidDiskThresholds { l1: u8, l2: u8, l3: u8 },

    #[error("Invalid retry backoff: initial ({initial} ms) must be <= max ({max} ms)")]
    InvalidRetryBackoff { initial: u64, max: u64 },

    #[error("Invalid custom regex pattern '{pattern}': {reason}")]
    InvalidRegexPattern { pattern: String, reason: String },

    #[error("Invalid server URL '{0}': URL must not be empty")]
    InvalidServerUrl(String),

    #[error("Invalid HTTP port: 0")]
    InvalidHttpPort,
}

pub trait Validate {
    fn validate(&self) -> Result<(), ConfigValidationError>;
}

impl Validate for RecorderConfig {
    fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.version == 0 {
            return Err(ConfigValidationError::InvalidVersion(self.version));
        }

        // Validate Upload Chunk Size
        if !(64..=256).contains(&self.upload.chunk_size_mb) {
            return Err(ConfigValidationError::InvalidChunkSize(self.upload.chunk_size_mb));
        }

        // Validate Capture Settings
        if self.capture.screenshot_quality == 0 || self.capture.screenshot_quality > 100 {
            return Err(ConfigValidationError::InvalidScreenshotQuality(
                self.capture.screenshot_quality,
            ));
        }

        if !(0.0..=1.0).contains(&self.capture.screenshot_diff_threshold) {
            return Err(ConfigValidationError::InvalidDiffThreshold(
                self.capture.screenshot_diff_threshold,
            ));
        }

        if self.capture.video_fps == 0 || self.capture.video_fps > 60 {
            return Err(ConfigValidationError::InvalidVideoFps(self.capture.video_fps));
        }

        if !(100..=50000).contains(&self.capture.video_bitrate_kbps) {
            return Err(ConfigValidationError::InvalidVideoBitrate(
                self.capture.video_bitrate_kbps,
            ));
        }

        // Validate Privacy
        if !(0.0..=8.0).contains(&self.privacy.entropy_threshold) {
            return Err(ConfigValidationError::InvalidEntropyThreshold(
                self.privacy.entropy_threshold,
            ));
        }

        for pattern in &self.privacy.custom_regex_patterns {
            if let Err(e) = Regex::new(pattern) {
                return Err(ConfigValidationError::InvalidRegexPattern {
                    pattern: pattern.clone(),
                    reason: e.to_string(),
                });
            }
        }

        // Validate Spool Disk Pressure Watermarks
        let l1 = self.spool.disk_pressure_level1_pct;
        let l2 = self.spool.disk_pressure_level2_pct;
        let l3 = self.spool.disk_pressure_level3_pct;
        if !(l1 < l2 && l2 < l3 && l3 <= 100) {
            return Err(ConfigValidationError::InvalidDiskThresholds { l1, l2, l3 });
        }

        // Validate Upload Retry Backoff
        if self.upload.initial_retry_backoff_ms > self.upload.max_retry_backoff_ms {
            return Err(ConfigValidationError::InvalidRetryBackoff {
                initial: self.upload.initial_retry_backoff_ms,
                max: self.upload.max_retry_backoff_ms,
            });
        }

        if self.upload.server_url.trim().is_empty() {
            return Err(ConfigValidationError::InvalidServerUrl(
                self.upload.server_url.clone(),
            ));
        }

        // Validate Server Port
        if self.server.http_port == 0 {
            return Err(ConfigValidationError::InvalidHttpPort);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_default_config() {
        let config = RecorderConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_chunk_size() {
        let mut config = RecorderConfig::default();
        config.upload.chunk_size_mb = 32;
        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::InvalidChunkSize(32))
        );
    }

    #[test]
    fn test_invalid_disk_watermarks() {
        let mut config = RecorderConfig::default();
        config.spool.disk_pressure_level1_pct = 90;
        config.spool.disk_pressure_level2_pct = 80;
        assert!(matches!(
            config.validate(),
            Err(ConfigValidationError::InvalidDiskThresholds { .. })
        ));
    }
}
