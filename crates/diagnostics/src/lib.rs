//! Unified diagnostics, structured JSON logging, metrics, and health inspection.

pub mod error_taxonomy;
pub mod health;
pub mod json_layer;
pub mod metrics;
pub mod subscriber;

pub use error_taxonomy::{DiagnosticEvent, DiagnosticSeverity, DiagnosticsError};
pub use health::{HealthProbe, HealthStatus, ProbeResult, SystemHealthAggregator, SystemHealthReport};
pub use json_layer::{JsonPrivacyFormatter, StructuredLogRecord};
pub use metrics::{LatencyMetrics, MetricsCollector, MetricsSnapshot, QueueDepths};
pub use subscriber::{init_diagnostics, DiagnosticsConfig, DiagnosticsGuard};
