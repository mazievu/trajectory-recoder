use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// 6-Stage Spool lifecycle states for session storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpoolState {
    Recording,
    Finalizing,
    PendingUpload,
    Uploading,
    Uploaded,
    Failed,
}

impl SpoolState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Finalizing => "finalizing",
            Self::PendingUpload => "pending_upload",
            Self::Uploading => "uploading",
            Self::Uploaded => "uploaded",
            Self::Failed => "failed",
        }
    }

    pub fn all() -> &'static [SpoolState] {
        &[
            Self::Recording,
            Self::Finalizing,
            Self::PendingUpload,
            Self::Uploading,
            Self::Uploaded,
            Self::Failed,
        ]
    }
}

impl fmt::Display for SpoolState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Manages atomic directory moves between spool stages.
#[derive(Clone)]
pub struct SpoolDirectoryManager {
    root: PathBuf,
}

impl SpoolDirectoryManager {
    pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        for state in SpoolState::all() {
            fs::create_dir_all(root.join(state.as_str()))?;
        }
        Ok(Self { root })
    }

    /// Root directory containing every spool lifecycle stage.
    pub fn base_path(&self) -> &Path {
        &self.root
    }

    pub fn recording_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::Recording)
    }

    pub fn finalizing_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::Finalizing)
    }

    pub fn pending_upload_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::PendingUpload)
    }

    pub fn uploading_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::Uploading)
    }

    pub fn uploaded_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::Uploaded)
    }

    pub fn failed_dir(&self) -> PathBuf {
        self.stage_path(SpoolState::Failed)
    }

    pub fn stage_path(&self, state: SpoolState) -> PathBuf {
        self.root.join(state.as_str())
    }

    pub fn session_path(&self, state: SpoolState, session_id: &str) -> PathBuf {
        self.stage_path(state).join(session_id)
    }

    /// Atomically transitions a session folder from `from` state to `to` state.
    pub fn transition(
        &self,
        session_id: &str,
        from: SpoolState,
        to: SpoolState,
    ) -> std::io::Result<PathBuf> {
        let src = self.session_path(from, session_id);
        let dst = self.session_path(to, session_id);

        if !src.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Session {} not found in stage {}", session_id, from),
            ));
        }

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&src, &dst)?;
        info!("Session {} transitioned: {} -> {}", session_id, from, to);
        Ok(dst)
    }

    /// List all session IDs currently residing in a specific spool state.
    pub fn list_sessions(&self, state: SpoolState) -> std::io::Result<Vec<String>> {
        let dir = self.stage_path(state);
        let mut sessions = Vec::new();
        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    sessions.push(name.to_string());
                }
            }
        }
        Ok(sessions)
    }

    /// Purge oldest uploaded sessions when disk space is required.
    pub fn purge_uploaded_older_than(&self, max_to_keep: usize) -> std::io::Result<usize> {
        let mut sessions = self.list_sessions(SpoolState::Uploaded)?;
        if sessions.len() <= max_to_keep {
            return Ok(0);
        }

        // Sort lexically (chronological order given session id format)
        sessions.sort();
        let to_remove = sessions.len() - max_to_keep;
        let mut purged = 0;

        for sid in sessions.iter().take(to_remove) {
            let path = self.session_path(SpoolState::Uploaded, sid);
            if fs::remove_dir_all(&path).is_ok() {
                purged += 1;
            }
        }

        Ok(purged)
    }
}
