use core_types::id::GlobalEventId;
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_BLOCK_SIZE: u64 = 10_000;
pub const PREALLOCATION_THRESHOLD: u64 = 1_000;

/// Crash-safe Global Event ID allocator with block reservation and atomic disk persistence.
pub struct GlobalEventIdAllocator {
    spool_root: PathBuf,
    block_size: u64,
    current_id: Arc<AtomicU64>,
    block_end: Arc<AtomicU64>,
    reserve_lock: Mutex<()>,
}

impl GlobalEventIdAllocator {
    pub fn new(spool_root: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::with_block_size(spool_root, DEFAULT_BLOCK_SIZE)
    }

    pub fn with_block_size(spool_root: impl AsRef<Path>, block_size: u64) -> std::io::Result<Self> {
        let root = spool_root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;

        let dat_path = root.join("global_event_id.dat");
        let last_reserved = if dat_path.exists() {
            let mut file = File::open(&dat_path)?;
            let mut buf = [0u8; 8];
            if file.read_exact(&mut buf).is_ok() {
                u64::from_le_bytes(buf)
            } else {
                0
            }
        } else {
            0
        };

        let new_block_end = last_reserved + block_size;
        Self::persist_reservation(&root, new_block_end)?;

        let start_id = last_reserved + 1;

        Ok(Self {
            spool_root: root,
            block_size,
            current_id: Arc::new(AtomicU64::new(start_id)),
            block_end: Arc::new(AtomicU64::new(new_block_end)),
            reserve_lock: Mutex::new(()),
        })
    }

    fn persist_reservation(spool_root: &Path, block_end: u64) -> std::io::Result<()> {
        let tmp_path = spool_root.join("global_event_id.dat.tmp");
        let dat_path = spool_root.join("global_event_id.dat");

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)?;

        file.write_all(&block_end.to_le_bytes())?;
        file.sync_all()?;
        drop(file);

        std::fs::rename(&tmp_path, &dat_path)?;
        Ok(())
    }

    /// Allocate the next monotonic Global Event ID.
    pub fn next_id(&self) -> u64 {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);
        let end = self.block_end.load(Ordering::Relaxed);

        let threshold = (self.block_size / 10).max(1).min(PREALLOCATION_THRESHOLD);
        if id + threshold >= end {
            let _guard = self.reserve_lock.lock();
            let current_end = self.block_end.load(Ordering::Relaxed);
            if id + threshold >= current_end {
                let next_end = current_end + self.block_size;
                if let Ok(()) = Self::persist_reservation(&self.spool_root, next_end) {
                    self.block_end.store(next_end, Ordering::SeqCst);
                }
            }
        }

        id
    }

    pub fn next_global_event_id(&self) -> GlobalEventId {
        GlobalEventId::new(self.next_id())
    }

    pub fn current_atomic(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.current_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_global_id_allocator_crash_safety() {
        let dir = tempdir().unwrap();
        let spool_root = dir.path();

        // 1. Initial run: allocates 1..100
        {
            let alloc = GlobalEventIdAllocator::with_block_size(spool_root, 100).unwrap();
            assert_eq!(alloc.next_id(), 1);
            assert_eq!(alloc.next_id(), 2);
            assert_eq!(alloc.next_id(), 3);
        }

        // 2. Simulated crash & restart: must skip remainder of first block and start at 101
        {
            let alloc = GlobalEventIdAllocator::with_block_size(spool_root, 100).unwrap();
            let id = alloc.next_id();
            assert_eq!(id, 101);
            assert_eq!(alloc.next_id(), 102);
        }

        // 3. Simulated second crash: must start at 201
        {
            let alloc = GlobalEventIdAllocator::with_block_size(spool_root, 100).unwrap();
            let id = alloc.next_id();
            assert_eq!(id, 201);
        }
    }
}
