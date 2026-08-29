use crate::schema::*;
use std::path::PathBuf;

pub fn default_recorder_config() -> RecorderConfig {
    RecorderConfig {
        version: 1,
        machine: default_machine_identity_config(),
        capture: default_capture_config(),
        privacy: default_privacy_config(),
        spool: default_spool_config(),
        upload: default_upload_config(),
        server: default_server_config(),
        diagnostics: default_diagnostics_config(),
    }
}

impl Default for RecorderConfig {
    fn default() -> Self {
        default_recorder_config()
    }
}

pub fn default_machine_identity_config() -> MachineIdentityConfig {
    MachineIdentityConfig {
        machine_id: String::new(),
        machine_name: String::new(),
        enrollment_token: None,
        device_token: None,
        employee_id: None,
        organization_id: None,
    }
}

impl Default for MachineIdentityConfig {
    fn default() -> Self {
        default_machine_identity_config()
    }
}

pub fn default_capture_config() -> CaptureConfig {
    CaptureConfig {
        mouse_enabled: true,
        keyboard_enabled: true,
        window_tracking_enabled: true,
        uia_enabled: true,
        uia_timeout_ms: 75,
        uia_max_depth: 3,
        screenshot_enabled: true,
        screenshot_quality: 80,
        screenshot_diff_threshold: 0.005, // 0.5% screen difference
        screenshot_stabilization_delays_ms: vec![200, 500, 1000],
        continuous_video: true,
        video_fps: 10,
        video_bitrate_kbps: 1500,
        video_keyframe_interval_secs: 2.0,
        video_hardware_accel: true,
        typing_burst_debounce_ms: 500,
        scroll_burst_debounce_ms: 300,
        drag_drop_distance_threshold_px: 5.0,
        clipboard_tracking_enabled: true,
        clipboard_metadata_only: true,
        file_events_enabled: true,
        user_idle_threshold_secs: 60,
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        default_capture_config()
    }
}

pub fn default_privacy_config() -> PrivacyConfig {
    PrivacyConfig {
        excluded_apps: vec![
            "1Password.exe".to_string(),
            "KeePass.exe".to_string(),
            "Bitwarden.exe".to_string(),
            "credentialui.exe".to_string(),
            "mstsc.exe".to_string(),
        ],
        excluded_domains: vec![
            "login.live.com".to_string(),
            "accounts.google.com".to_string(),
            "auth0.com".to_string(),
            "okta.com".to_string(),
            "chase.com".to_string(),
            "wellsfargo.com".to_string(),
        ],
        excluded_window_titles: vec![
            "Windows Security".to_string(),
            "User Account Control".to_string(),
            "Enter credentials".to_string(),
        ],
        redact_credit_cards: true,
        redact_ssn: true,
        redact_api_keys: true,
        redact_jwt: true,
        redact_high_entropy: true,
        entropy_threshold: 4.5,
        entropy_min_length: 16,
        mask_unobserved_text: true,
        custom_regex_patterns: Vec::new(),
        fail_closed: true,
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        default_privacy_config()
    }
}

pub fn default_spool_config() -> SpoolConfig {
    SpoolConfig {
        spool_dir: PathBuf::from(r"C:\ProgramData\TrajectoryRecorder\spool"),
        local_retention_hours: 72,
        disk_pressure_level1_pct: 70,
        disk_pressure_level2_pct: 85,
        disk_pressure_level3_pct: 92,
        ndjson_flush_interval_ms: 2000,
        ndjson_buffer_capacity_kb: 64,
    }
}

impl Default for SpoolConfig {
    fn default() -> Self {
        default_spool_config()
    }
}

pub fn default_upload_config() -> UploadConfig {
    UploadConfig {
        server_url: "http://127.0.0.1:8080".to_string(),
        chunk_size_mb: 64,
        max_retries: 10,
        initial_retry_backoff_ms: 1000,
        max_retry_backoff_ms: 60000,
        jitter_factor: 0.25,
        upload_concurrency: 2,
        bandwidth_limit_kbps: None,
        retry_oldest_first: true,
    }
}

impl Default for UploadConfig {
    fn default() -> Self {
        default_upload_config()
    }
}

pub fn default_server_config() -> ServerConfig {
    ServerConfig {
        http_port: 8080,
        http_host: "0.0.0.0".to_string(),
        // SECURITY: These fields must never have production values in source code.
        // Load from environment variables or Windows Credential Manager.
        database_url: String::new(), // Must be set via DATABASE_URL env var
        s3_endpoint: "http://localhost:9000".to_string(),
        s3_bucket: "trajectory-archives".to_string(),
        s3_region: "us-east-1".to_string(),
        s3_access_key: String::new(), // Must be set via S3_ACCESS_KEY env var
        s3_secret_key: String::new(), // Must be set via S3_SECRET_KEY env var
        jwt_secret: String::new(),    // Must be set via JWT_SECRET env var
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        default_server_config()
    }
}

pub fn default_diagnostics_config() -> DiagnosticsConfig {
    DiagnosticsConfig {
        log_level: "info".to_string(),
        log_to_file: true,
        log_dir: PathBuf::from(r"C:\ProgramData\TrajectoryRecorder\logs"),
        metrics_export_interval_secs: 60,
    }
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        default_diagnostics_config()
    }
}
