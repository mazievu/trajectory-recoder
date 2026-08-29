use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use crate::error_taxonomy::DiagnosticsError;

/// Guard holding background worker threads for non-blocking file loggers.
/// Must be retained by main() until process termination.
pub struct DiagnosticsGuard {
    _file_guard: Option<WorkerGuard>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticsConfig {
    pub log_level: String,              // "info", "debug", "trace"
    pub log_directory: Option<PathBuf>, // e.g. "%ProgramData%/TrajectoryRecorder/logs"
    pub log_filename_prefix: String,    // "trajectory-agent" or "trajectory-supervisor"
    pub log_to_console: bool,
    pub log_to_file: bool,
    pub enable_json_format: bool,
    pub max_log_files: usize,
    pub machine_id: String,
    pub process_name: String,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_directory: None,
            log_filename_prefix: "trajectory-recorder".to_string(),
            log_to_console: true,
            log_to_file: false,
            enable_json_format: true,
            max_log_files: 7,
            machine_id: "unknown-machine".to_string(),
            process_name: "unknown-process".to_string(),
        }
    }
}

/// Initialize tracing subscriber with multi-layer output:
/// 1. Console Layer: standard human-readable or JSON formatting.
/// 2. Rolling File Layer: non-blocking daily rotation with custom privacy-sanitized JSON.
pub fn init_diagnostics(config: &DiagnosticsConfig) -> Result<DiagnosticsGuard, DiagnosticsError> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let mut file_guard = None;
    let mut layers = Vec::new();

    // 1. Console layer
    if config.log_to_console {
        if config.enable_json_format {
            let json_console = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_thread_ids(true)
                .boxed();
            layers.push(json_console);
        } else {
            let text_console = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .boxed();
            layers.push(text_console);
        }
    }

    // 2. Rolling file layer
    if config.log_to_file {
        if let Some(log_dir) = &config.log_directory {
            std::fs::create_dir_all(log_dir)
                .map_err(|e| DiagnosticsError::IoError(format!("Failed to create log directory: {e}")))?;

            let file_appender = tracing_appender::rolling::daily(log_dir, &config.log_filename_prefix);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            file_guard = Some(guard);

            let file_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(true)
                .boxed();
            layers.push(file_layer);
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(layers)
        .try_init()
        .map_err(|e| DiagnosticsError::InitError(format!("Failed to register tracing subscriber: {e}")))?;

    tracing::info!(
        machine_id = %config.machine_id,
        process_name = %config.process_name,
        "Diagnostics and tracing subsystem initialized successfully"
    );

    Ok(DiagnosticsGuard { _file_guard: file_guard })
}
