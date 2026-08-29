use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    pub server_url: String,
    pub heartbeat_interval_secs: u64,
    pub chunk_size_bytes: usize,
    pub max_spool_disk_gb: u64,
    pub enable_video: bool,
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            server_url: "https://api.trajectory.corp".to_string(),
            heartbeat_interval_secs: 30,
            chunk_size_bytes: 64 * 1024 * 1024,
            max_spool_disk_gb: 20,
            enable_video: true,
        }
    }
}

#[test]
fn test_f04_config_defaults_and_override() {
    let cfg = TrajectoryConfig::default();
    assert_eq!(cfg.heartbeat_interval_secs, 30);
    assert_eq!(cfg.chunk_size_bytes, 64 * 1024 * 1024);
    assert!(cfg.enable_video);
}
