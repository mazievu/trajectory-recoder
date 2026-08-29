use session::GlobalEventIdAllocator;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_allocator_multithreaded_concurrency_no_duplicates_no_skips() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    let allocator = Arc::new(GlobalEventIdAllocator::with_block_size(spool_root, 1000).unwrap());

    let thread_count = 20;
    let allocs_per_thread = 5000;
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let alloc_clone = Arc::clone(&allocator);
        let handle = thread::spawn(move || {
            let mut ids = Vec::with_capacity(allocs_per_thread);
            for _ in 0..allocs_per_thread {
                ids.push(alloc_clone.next_id());
            }
            ids
        });
        handles.push(handle);
    }

    let mut all_ids = Vec::with_capacity(thread_count * allocs_per_thread);
    for handle in handles {
        let ids = handle.join().expect("Thread should not panic");
        all_ids.extend(ids);
    }

    assert_eq!(
        all_ids.len(),
        thread_count * allocs_per_thread,
        "Total allocated IDs count must match expected"
    );

    let unique_ids: HashSet<u64> = all_ids.iter().copied().collect();
    assert_eq!(
        unique_ids.len(),
        all_ids.len(),
        "Must have zero duplicate IDs across concurrent threads"
    );

    // Verify all IDs from 1 to total are present
    let min_id = *all_ids.iter().min().unwrap();
    let max_id = *all_ids.iter().max().unwrap();
    assert_eq!(min_id, 1, "First allocated ID must be 1");
    assert_eq!(
        max_id,
        (thread_count * allocs_per_thread) as u64,
        "Max allocated ID must equal total count"
    );

    // Verify disk state covers all allocated IDs
    let dat_path = spool_root.join("global_event_id.dat");
    assert!(dat_path.exists());
    let mut file = File::open(&dat_path).unwrap();
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).unwrap();
    let reserved_on_disk = u64::from_le_bytes(buf);

    assert!(
        reserved_on_disk >= max_id,
        "Disk reservation ({}) must be >= max allocated ID ({})",
        reserved_on_disk,
        max_id
    );
}

#[test]
fn test_allocator_block_exhaustion_rapid_resizing() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    // Use tiny block size of 7 to force rapid block exhaustion
    let block_size = 7;
    let allocator =
        Arc::new(GlobalEventIdAllocator::with_block_size(spool_root, block_size).unwrap());

    let thread_count = 16;
    let allocs_per_thread = 250; // Total 4000 allocations -> ~571 blocks reserved
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let alloc_clone = Arc::clone(&allocator);
        let handle = thread::spawn(move || {
            let mut ids = Vec::with_capacity(allocs_per_thread);
            for _ in 0..allocs_per_thread {
                ids.push(alloc_clone.next_id());
            }
            ids
        });
        handles.push(handle);
    }

    let mut all_ids = Vec::with_capacity(thread_count * allocs_per_thread);
    for handle in handles {
        let ids = handle.join().expect("Thread should not panic");
        all_ids.extend(ids);
    }

    let unique_ids: HashSet<u64> = all_ids.iter().copied().collect();
    assert_eq!(unique_ids.len(), 4000);
    assert_eq!(*all_ids.iter().min().unwrap(), 1);
    assert_eq!(*all_ids.iter().max().unwrap(), 4000);

    // Verify dat file
    let dat_path = spool_root.join("global_event_id.dat");
    let mut file = File::open(&dat_path).unwrap();
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).unwrap();
    let reserved_on_disk = u64::from_le_bytes(buf);
    assert!(
        reserved_on_disk >= 4000,
        "Disk reservation ({}) must be >= 4000",
        reserved_on_disk
    );
}

#[test]
fn test_allocator_extreme_contention_tiny_block() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    // Extreme case: block size = 3 with 30 threads allocating 100 IDs each
    let block_size = 3;
    let allocator =
        Arc::new(GlobalEventIdAllocator::with_block_size(spool_root, block_size).unwrap());

    let thread_count = 30;
    let allocs_per_thread = 100;
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let alloc_clone = Arc::clone(&allocator);
        handles.push(thread::spawn(move || {
            let mut ids = Vec::new();
            for _ in 0..allocs_per_thread {
                ids.push(alloc_clone.next_id());
            }
            ids
        }));
    }

    let mut all_ids = Vec::new();
    for h in handles {
        all_ids.extend(h.join().unwrap());
    }

    let total = thread_count * allocs_per_thread;
    let unique: HashSet<u64> = all_ids.iter().copied().collect();
    assert_eq!(unique.len(), total);
    assert_eq!(*all_ids.iter().min().unwrap(), 1);
    assert_eq!(*all_ids.iter().max().unwrap(), total as u64);
}

#[test]
fn test_allocator_crash_recovery_sequence() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    let mut previous_max_id = 0u64;
    let runs = 10;
    let block_size = 50;

    for run in 0..runs {
        // Instantiate new allocator (simulating restart after crash)
        let allocator = GlobalEventIdAllocator::with_block_size(spool_root, block_size).unwrap();

        let alloc_count = (run + 1) * 3; // Variable number of IDs per run
        let mut ids = Vec::new();
        for _ in 0..alloc_count {
            ids.push(allocator.next_id());
        }

        let first_id = *ids.first().unwrap();
        let last_id = *ids.last().unwrap();

        if run > 0 {
            assert!(
                first_id > previous_max_id,
                "Run {}: first_id ({}) must be strictly greater than previous_max_id ({}) to prevent collision",
                run,
                first_id,
                previous_max_id
            );
        }

        previous_max_id = last_id;
    }
}

#[test]
fn test_allocator_corrupted_dat_file_recovery() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    // 1. Create a 0-byte file (e.g. crash right after file creation before write)
    let dat_path = spool_root.join("global_event_id.dat");
    File::create(&dat_path).unwrap();

    let allocator = GlobalEventIdAllocator::with_block_size(spool_root, 100).unwrap();
    assert_eq!(allocator.next_id(), 1);
    assert_eq!(allocator.next_id(), 2);

    // 2. Truncate file to 3 bytes (corrupted header)
    {
        let mut file = File::create(&dat_path).unwrap();
        file.write_all(&[1, 2, 3]).unwrap();
    }

    let allocator2 = GlobalEventIdAllocator::with_block_size(spool_root, 100).unwrap();
    assert_eq!(allocator2.next_id(), 1);
    assert_eq!(allocator2.next_id(), 2);
}

#[test]
fn test_allocator_crash_after_concurrency_preserves_monotonicity() {
    let dir = tempdir().unwrap();
    let spool_root = dir.path();

    let max_id_allocated = {
        let allocator = Arc::new(GlobalEventIdAllocator::with_block_size(spool_root, 50).unwrap());
        let thread_count = 10;
        let per_thread = 100;
        let mut handles = Vec::new();

        for _ in 0..thread_count {
            let a = Arc::clone(&allocator);
            handles.push(thread::spawn(move || {
                let mut max = 0;
                for _ in 0..per_thread {
                    let id = a.next_id();
                    if id > max {
                        max = id;
                    }
                }
                max
            }));
        }

        let mut overall_max = 0;
        for h in handles {
            let m = h.join().unwrap();
            if m > overall_max {
                overall_max = m;
            }
        }
        overall_max
    };

    assert_eq!(max_id_allocated, 1000);

    // Now simulate crash and restart
    let allocator_restart = GlobalEventIdAllocator::with_block_size(spool_root, 50).unwrap();
    let next_id_after_restart = allocator_restart.next_id();

    assert!(
        next_id_after_restart > max_id_allocated,
        "After crash/restart, new ID ({}) must be strictly greater than previous max allocated ID ({})",
        next_id_after_restart,
        max_id_allocated
    );
}
