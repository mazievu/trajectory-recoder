use crate::manager::ConfigManager;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// File watcher that listens for `config.toml` modifications and triggers atomic reload.
pub struct ConfigFileWatcher {
    _watcher: RecommendedWatcher,
}

impl ConfigFileWatcher {
    /// Starts watching the specified configuration file for real-time changes.
    pub fn start<F>(
        config_path: PathBuf,
        manager: Arc<ConfigManager>,
        on_reloaded: F,
    ) -> Result<Self, notify::Error>
    where
        F: Fn(Arc<crate::schema::RecorderConfig>) + Send + Sync + 'static,
    {
        let canonical_path = config_path.canonicalize().unwrap_or_else(|_| config_path.clone());
        let mut last_event_time = Instant::now() - Duration::from_secs(10);
        let debounce_duration = Duration::from_millis(500);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    let is_modify_or_create = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    );

                    if is_modify_or_create && event.paths.iter().any(|p| p.ends_with(&canonical_path) || p == &canonical_path) {
                        let now = Instant::now();
                        if now.duration_since(last_event_time) >= debounce_duration {
                            last_event_time = now;
                            info!("Detected configuration file change, reloading...");
                            match manager.reload() {
                                Ok(()) => {
                                    let new_cfg = manager.get();
                                    on_reloaded(new_cfg);
                                }
                                Err(err) => {
                                    error!(error = %err, "Failed to reload updated configuration; retaining current safe config");
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!(error = %err, "Config file watcher notification error");
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        let watch_target = if config_path.is_file() {
            config_path.parent().unwrap_or_else(|| Path::new("."))
        } else {
            &config_path
        };

        watcher.watch(watch_target, RecursiveMode::NonRecursive)?;
        info!(target = %watch_target.display(), "Config file watcher initialized");

        Ok(Self { _watcher: watcher })
    }
}
