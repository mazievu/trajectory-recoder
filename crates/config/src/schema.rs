use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Root configuration tree for Trajectory Recorder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecorderConfig {
    /// Configuration schema version.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Machine identity and registration tokens.
    #[serde(default)]
    pub machine: MachineIdentityConfig,
    /// Capture subsystems policies and thresholds.
    #[serde(default)]
    pub capture: CaptureConfig,
    /// Privacy and redaction engine policies.
    #[serde(default)]
    pub privacy: PrivacyConfig,
    /// Local spool storage and disk pressure rules.
    #[serde(default)]
    pub spool: SpoolConfig,
    /// Resumable chunked upload client settings.
    #[serde(default)]
    pub upload: UploadConfig,
    /// Ingestion server connectivity parameters.
    #[serde(default)]
    pub server: ServerConfig,
    /// Logging and metrics diagnostics settings.
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdentityConfig {
    pub machine_id: String,
    pub machine_name: String,
    pub enrollment_token: Option<String>,
    pub device_token: Option<String>,
    pub employee_id: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub mouse_enabled: bool,
    pub keyboard_enabled: bool,
    pub window_tracking_enabled: bool,
    pub uia_enabled: bool,
    pub uia_timeout_ms: u64,
    pub uia_max_depth: u32,
    pub screenshot_enabled: bool,
    pub screenshot_quality: u8,
    pub screenshot_diff_threshold: f32,
    pub screenshot_stabilization_delays_ms: Vec<u64>,
    pub continuous_video: bool,
    pub video_fps: u32,
    pub video_bitrate_kbps: u32,
    pub video_keyframe_interval_secs: f32,
    pub video_hardware_accel: bool,
    pub typing_burst_debounce_ms: u64,
    pub scroll_burst_debounce_ms: u64,
    pub drag_drop_distance_threshold_px: f64,
    pub clipboard_tracking_enabled: bool,
    pub clipboard_metadata_only: bool,
    pub file_events_enabled: bool,
    pub user_idle_threshold_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub excluded_apps: Vec<String>,
    pub excluded_domains: Vec<String>,
    pub excluded_window_titles: Vec<String>,
    pub redact_credit_cards: bool,
    pub redact_ssn: bool,
    pub redact_api_keys: bool,
    pub redact_jwt: bool,
    pub redact_high_entropy: bool,
    pub entropy_threshold: f64,
    pub entropy_min_length: usize,
    pub mask_unobserved_text: bool,
    pub custom_regex_patterns: Vec<String>,
    pub fail_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpoolConfig {
    pub spool_dir: PathBuf,
    pub local_retention_hours: u32,
    pub disk_pressure_level1_pct: u8,
    pub disk_pressure_level2_pct: u8,
    pub disk_pressure_level3_pct: u8,
    pub ndjson_flush_interval_ms: u64,
    pub ndjson_buffer_capacity_kb: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UploadConfig {
    pub server_url: String,
    pub chunk_size_mb: usize,
    pub max_retries: u32,
    pub initial_retry_backoff_ms: u64,
    pub max_retry_backoff_ms: u64,
    pub jitter_factor: f64,
    pub upload_concurrency: usize,
    pub bandwidth_limit_kbps: Option<u32>,
    pub retry_oldest_first: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub http_port: u16,
    pub http_host: String,
    pub database_url: String,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub jwt_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsConfig {
    pub log_level: String,
    pub log_to_file: bool,
    pub log_dir: PathBuf,
    pub metrics_export_interval_secs: u64,
}
