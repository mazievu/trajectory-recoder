# Original User Request

## Initial Request — 2026-08-29T03:48:19+07:00

You are the Top-Level Project Orchestrator (teamwork_preview_orchestrator).
Your mission is to lead and execute the complete, production-grade build of the **Trajectory Recorder** system on Windows in Rust (Edition 2024, Win32/windows-rs, UI Automation), Tauri 2 + React + TypeScript, Chrome/Edge Companion Extension, and Axum Ingestion Server strictly following the Master Implementation Specification (Sections 0–75) and 40 Acceptance Criteria.

### Workspace & Spec Files:
- Workspace root: `d:/tools GTF/trajectory recoder`
- Original user request: `C:/Users/admin/.gemini/antigravity/brain/30be3812-894c-4e5a-a03c-350b13028e3e/ORIGINAL_REQUEST.md`
- Project specification files in workspace: `spec.md`, `producttechnical requirement.md`

### Responsibilities:
1. Decompose the project across all 10 Phases:
   - Phase 1 — Foundation (Cargo workspace, `core-types`, dual timestamps, config, IPC, DPAPI)
   - Phase 2 — Capture Core (Win32 input hooks, mouse/keyboard, active window, Event Bus, NDJSON writer)
   - Phase 3 — Semantic Capture (UIA, typing/scroll burst grouping, drag&drop, Privacy Engine redaction)
   - Phase 4 — State Evidence (Multi-monitor WebP screenshots, state change diffs, video fragments)
   - Phase 5 — Browser Companion (MV3 extension, native host bridge, DOM selectors/events)
   - Phase 6 — Session Engine & Persistence (Global ID, hourly rotation, SQLite WAL, crash recovery)
   - Phase 7 — Upload Pipeline & Spool (Spool state transitions, TAR+Zstd, XChaCha20-Poly1305, chunking, upload client)
   - Phase 8 — Ingestion Server & Ingestion API (Axum, PostgreSQL migrations, S3/MinIO storage, chunk verification)
   - Phase 9 — Desktop UI & Interactive Trajectory Viewer (Tauri 2, React, TypeScript, step-by-step viewer, diffs, search)
   - Phase 10 — Hardening & Full Test Suite (Harness app, 4-tier disk protection, recovery tests, 30-min E2E verification)
2. Ensure all 5 binaries, 19 crates, browser extension, server migrations, and test suites are fully implemented, compiling, and passing tests.
3. Verify all 40 Production Acceptance Criteria (Definition of Done).
4. Maintain `plan.md`, `progress.md`, and `BRIEFING.md` in your agent folder.

## Follow-up — 2026-08-29T03:02:26Z

You are a Senior Rust/Windows Systems Engineer. Your task is to fix a partially-implemented production codebase so it compiles cleanly and satisfies all 40 Production Acceptance Criteria.

Working directory: d:/tools GTF/trajectory recoder
Integrity mode: development

## Context

The codebase has been independently audited. It has 3,980 nodes / 9,893 edges indexed and a solid schema foundation, but fails to build and does not meet Production DoD due to the following verified critical issues.

---

## Priority 1 — BUILD BLOCKERS (Fix first, nothing else compiles until these are done)

### P1.1 — Invalid `use` syntax with hyphens in crate names

Rust `use` statements require underscores, not hyphens, even if `Cargo.toml` declares crate names with hyphens.

In `apps/capture-agent/src/main.rs`:
- Line 10: `use event-bus::bus::{EventBus, EventBusConfig};` → MUST be `use event_bus::bus::{EventBus, EventBusConfig};`
- Line 22: `use uia-win::inspector::UiaInspector;` → `use uia_win::inspector::UiaInspector;`
- Line 23: `use window-win::tracker::WindowTracker;` → `use window_win::tracker::WindowTracker;`

In `apps/uploader/src/main.rs`:
- Line 10: `use upload-client::{InitiateSessionRequest, UploadClient};` → `use upload_client::{InitiateSessionRequest, UploadClient};`

Fix all occurrences across the entire workspace. Search for any other `use X-Y::` patterns and fix them.

### P1.2 — References to non-existent enum variants in capture-agent

In `apps/capture-agent/src/main.rs` lines 146-153, the code references:
- `RawEventPayload::Input(ref inp)` — **does NOT exist** in `crates/core-types/src/event.rs`
- `RawInputPayload::Mouse(ref m)` — **does NOT exist**

The actual enum in `core-types/src/event.rs` line 72 is:
```rust
pub enum RawEventPayload {
    Mouse(RawMouseEvent),
    Keyboard(RawKeyboardEvent),
    Window(RawWindowEvent),
    UiAutomation(RawUiaEvent),
    Browser(RawBrowserEvent),
    Clipboard(RawClipboardEvent),
    File(RawFileEvent),
    Screen(RawScreenEvent),
    System(RawSystemEvent),
    Session(RawSessionEvent),
}
```

Fix the UIA inspection block in `capture-agent/src/main.rs` to use `RawEventPayload::Mouse(ref m)` directly with `m.physical_x` and `m.physical_y`.

### P1.3 — Remove non-existent types from `session/src/ndjson.rs`

Check `crates/session/src/ndjson.rs` around line 93. Remove any references to `RawSummary` or `sequence_number` that do not exist in `core-types`. Fix to use only types that exist in core-types schema.

### P1.4 — Fix missing dependencies in Cargo.toml files

Add to `apps/capture-agent/Cargo.toml` under `[dependencies]`:
```toml
serde_json = { workspace = true }
tracing-subscriber = { workspace = true }
```

Add to `apps/uploader/Cargo.toml` under `[dependencies]`:
```toml
serde_json = { workspace = true }
tracing-subscriber = { workspace = true }
hex = { workspace = true }
```

Add to `apps/server/Cargo.toml` under `[dependencies]`:
```toml
parking_lot = { workspace = true }
uuid = { workspace = true }
hex = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
tracing-subscriber = { workspace = true }
serde_json = { workspace = true }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
object_store = { version = "0.11", features = ["aws"] }
jsonwebtoken = "9"
dotenvy = "0.15"
```

Add to `apps/supervisor/Cargo.toml` under `[dependencies]`:
```toml
sysinfo = { workspace = true }
tracing-subscriber = { workspace = true }
windows = { workspace = true }
```

Add to workspace `Cargo.toml` under `[workspace.dependencies]` if not already present:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
object_store = { version = "0.11", features = ["aws"] }
jsonwebtoken = "9"
dotenvy = "0.15"
windows-service = "0.7"
```

Also add `windows-service` to `apps/supervisor/Cargo.toml`.

---

## Priority 2 — SESSION PERSISTENCE GAPS

### P2.1 — Crash-safe Global Event ID

In `apps/capture-agent/src/main.rs` line 33:
```rust
let global_seq = Arc::new(AtomicU64::new(1));
```
This resets to 1 on every restart — not crash-safe, not cross-session continuous.

Replace with a crash-safe counter that:
1. Reads a `{spool_root}/global_event_id.dat` file on startup containing the last reserved block start (u64, little-endian 8 bytes).
2. Reserves a block of 10,000 IDs by writing `last_reserved + 10000` to the file atomically (write to `.tmp`, then rename).
3. Starts the in-memory `AtomicU64` from `last_reserved + 1`.
4. When within 1000 IDs of block end, pre-allocates next block in background.
5. On first run (file missing), starts from 1 and creates the file.

Create `crates/session/src/global_id.rs` implementing `GlobalEventIdAllocator` with this logic.

### P2.2 — Session directory must create all required subdirectories and files

In `crates/session/src/manager.rs`, the `start()` function only creates the session dir and opens `events.raw.ndjson` and `session.db`.

Must also:
1. Create subdirectories: `screenshots/`, `video/`, `browser/`, `uia/`, `diagnostics/` inside the active session dir.
2. Open a second NDJSON writer for `events.normalized.ndjson` (for canonical actions).
3. Write `manifest.json` on session open with initial metadata (schema version, session_id, machine_id, user_id, started_at, status: "RECORDING").
4. On session finalization/rotation, update `manifest.json` with ended_at, event_count, action_count, status: "FINALIZED".

`manifest.json` schema:
```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0",
  "session_id": "...",
  "machine_id": "...",
  "user_id": "...",
  "started_at": "...",
  "ended_at": null,
  "status": "RECORDING",
  "event_count": 0,
  "action_count": 0
}
```

Also: write canonical actions to `events.normalized.ndjson` instead of (or in addition to) only SQLite.

### P2.3 — Fix rotation error handling

In `crates/session/src/manager.rs` function `rotate_session()` (around lines 105-148):

Every `let _ = ...` that suppresses errors must be replaced with proper error handling:
```rust
// BEFORE (wrong):
let _ = self.db.checkpoint_wal();
let _ = self.ndjson_writer.flush_sync();
let _ = std::fs::rename(&self.active_dir, &finalizing_dir);

// AFTER (correct):
self.ndjson_writer.flush_sync().map_err(|e| {
    tracing::error!("Failed to flush NDJSON on rotation: {}", e);
    e
})?;
self.db.checkpoint_wal().map_err(|e| {
    tracing::error!("Failed to WAL checkpoint on rotation: {}", e);
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
})?;
std::fs::rename(&self.active_dir, &finalizing_dir).map_err(|e| {
    tracing::error!("Failed to rename session dir on rotation: {}", e);
    e
})?;
```

Also fix `finalize_session_meta` suppression similarly.

---

## Priority 3 — UPLOADER: Real HTTP Chunk Upload

### P3.1 — Implement real upload loop in `apps/uploader/src/main.rs`

Replace the mock upload (lines 56-62 with comment "In production") with a real implementation:

```rust
// After chunking succeeds:
let _ = spool_mgr.transition(&sid, SpoolState::PendingUpload, SpoolState::Uploading);

// Initiate session on server
let initiate_req = upload_client::InitiateSessionRequest {
    session_id: sid.clone(),
    machine_id: manifest.machine_id.clone(),
    chunk_count: manifest.chunk_count,
    total_size_bytes: manifest.total_size_bytes,
    archive_sha256: manifest.archive_sha256.clone(),
    schema_version: "1.0".to_string(),
};
match client.initiate_session(&initiate_req).await {
    Ok(_) => {},
    Err(e) => {
        error!("Failed to initiate session {}: {}", sid, e);
        // Will retry next loop
        continue;
    }
}

// Upload each chunk
let mut all_ok = true;
for chunk_idx in 0..manifest.chunk_count {
    let chunk_path = chunks_dir.join(format!("chunk_{:05}.bin", chunk_idx));
    match client.upload_chunk_with_retry(&sid, chunk_idx, &chunk_path, &manifest.chunk_sha256s[chunk_idx]).await {
        Ok(_) => info!("Chunk {}/{} uploaded for session {}", chunk_idx + 1, manifest.chunk_count, sid),
        Err(e) => {
            error!("Failed to upload chunk {} for session {}: {}", chunk_idx, sid, e);
            all_ok = false;
            break;
        }
    }
}

if all_ok {
    // Complete session and wait for SESSION_ACCEPTED
    match client.complete_session(&sid).await {
        Ok(resp) if resp.status == "SESSION_ACCEPTED" => {
            let _ = spool_mgr.transition(&sid, SpoolState::Uploading, SpoolState::Uploaded);
            info!("Session {} accepted by server.", sid);
        }
        Ok(resp) => {
            warn!("Server returned unexpected status '{}' for session {}", resp.status, sid);
            let _ = spool_mgr.transition(&sid, SpoolState::Uploading, SpoolState::Failed);
        }
        Err(e) => {
            error!("Failed to complete session {}: {}", sid, e);
            // Leave in Uploading state to retry
        }
    }
}
```

### P3.2 — Implement `upload_chunk_with_retry` in `crates/upload-client/`

Implement with exponential backoff + jitter:
```rust
pub async fn upload_chunk_with_retry(
    &self,
    session_id: &str,
    chunk_index: usize,
    chunk_path: &Path,
    expected_sha256: &str,
) -> Result<(), UploadError> {
    let chunk_data = tokio::fs::read(chunk_path).await?;
    let computed_sha256 = hex::encode(sha2::Sha256::digest(&chunk_data));
    
    let mut backoff_ms = 1000u64;
    for attempt in 0..self.config.max_retries {
        let res = self.http_client
            .put(format!("{}/api/v1/sessions/{}/chunks/{}", self.server_url, session_id, chunk_index))
            .header("X-Chunk-SHA256", &computed_sha256)
            .header("X-Chunk-Size", chunk_data.len().to_string())
            .body(chunk_data.clone())
            .send()
            .await;
        
        match res {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(resp) => {
                warn!("Chunk upload attempt {} failed: HTTP {}", attempt + 1, resp.status());
            }
            Err(e) => {
                warn!("Chunk upload attempt {} error: {}", attempt + 1, e);
            }
        }
        
        // Exponential backoff with jitter
        let jitter = (rand::random::<f64>() * 0.25 * backoff_ms as f64) as u64;
        tokio::time::sleep(Duration::from_millis(backoff_ms + jitter)).await;
        backoff_ms = (backoff_ms * 2).min(60_000);
    }
    Err(UploadError::MaxRetriesExceeded)
}
```

---

## Priority 4 — SERVER: PostgreSQL + S3 + Real Verification

### P4.1 — Replace in-memory HashMap state with PostgreSQL + S3

Rewrite `apps/server/src/main.rs` completely. The new server must:

1. **Read config from environment variables only** (never from hardcoded defaults):
   - `DATABASE_URL` — PostgreSQL connection string
   - `S3_ENDPOINT`, `S3_BUCKET`, `S3_REGION`, `S3_ACCESS_KEY`, `S3_SECRET_KEY` — S3/MinIO config
   - `JWT_SECRET` — JWT signing secret
   - `BIND_ADDR` — optional, default `0.0.0.0:8080`
   Load with `dotenvy::dotenv().ok()` for dev convenience.

2. **AppState** must be:
   ```rust
   #[derive(Clone)]
   pub struct AppState {
       pub db: sqlx::PgPool,
       pub object_store: Arc<dyn object_store::ObjectStore>,
       pub jwt_secret: String,
       pub s3_bucket: String,
   }
   ```

3. **Machine registration** (`POST /api/v1/machines/register`):
   - Upsert machine record in PostgreSQL `machines` table (idempotent on machine_id)
   - Sign a real JWT with `jsonwebtoken` crate using `JWT_SECRET` env var
   - Return `{ "machine_id": "...", "device_token": "<real_jwt>" }`

4. **Session initiation** (`POST /api/v1/sessions`):
   - Upsert session record in PostgreSQL `sessions` table with `ON CONFLICT (machine_id, session_id) DO NOTHING` for idempotency
   - Return session upload parameters

5. **Chunk upload** (`PUT /api/v1/sessions/{session_id}/chunks/{index}`):
   - Verify `X-Chunk-SHA256` header against actual SHA-256 of received body
   - **Stream chunk directly to S3/MinIO** at key: `trajectory/{machine_id}/{YYYY}/{MM}/{DD}/{HH}/{session_id}/chunk_{index:05}.bin`
   - Upsert chunk record in `session_chunks` table
   - Return success

6. **Complete session** (`POST /api/v1/sessions/{session_id}/complete`):
   - Verify all expected chunks have been received (query `session_chunks` table)
   - Read all chunks from S3 in order
   - Compute SHA-256 of concatenated chunks
   - Compare with stored `archive_sha256` from session initiation
   - Only if hash matches: update `sessions.status = 'ACCEPTED'`, return `{ "status": "SESSION_ACCEPTED" }`
   - If hash mismatch: return HTTP 422 with error

### P4.2 — Remove all hardcoded secrets from config defaults

In `crates/config/src/defaults.rs`, replace:
```rust
database_url: "postgres://postgres:postgres@localhost:5432/trajectory_db".to_string(),
s3_access_key: "minioadmin".to_string(),
s3_secret_key: "minioadmin".to_string(),
jwt_secret: "insecure-development-secret-change-in-production".to_string(),
```
With:
```rust
database_url: String::new(), // Must be set via DATABASE_URL env var
s3_access_key: String::new(), // Must be set via S3_ACCESS_KEY env var
s3_secret_key: String::new(), // Must be set via S3_SECRET_KEY env var
jwt_secret: String::new(), // Must be set via JWT_SECRET env var
```
Add a `// SECURITY: These fields must never have production values in source code.
// Load from environment variables or Windows Credential Manager.` comment.

### P4.3 — Create `.env.example` file for development

Create `server/.env.example`:
```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/trajectory_db
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=trajectory-archives
S3_REGION=us-east-1
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
JWT_SECRET=change-this-in-production-use-a-long-random-secret
BIND_ADDR=0.0.0.0:8080
```

Also add `.env` to `.gitignore` in repo root.

---

## Priority 5 — SCREENSHOT: Real WebP Pipeline

### P5.1 — Encode BGRA to WebP in `crates/capture-win/src/screenshot.rs`

Add `image` crate to `capture-win/Cargo.toml` with WebP support:
```toml
image = { version = "0.25", default-features = false, features = ["webp"] }
```

After capturing BGRA buffer (the existing `capture_screen_gdi` function), add WebP encoding:
```rust
// After getting BGRA buffer:
pub fn encode_webp(bgra_data: &[u8], width: u32, height: u32, quality: u8) -> Option<Vec<u8>> {
    // Convert BGRA -> RGBA for image crate
    let mut rgba = Vec::with_capacity(bgra_data.len());
    for pixel in bgra_data.chunks_exact(4) {
        rgba.push(pixel[2]); // R
        rgba.push(pixel[1]); // G  
        rgba.push(pixel[0]); // B
        rgba.push(pixel[3]); // A
    }
    let img = image::RgbaImage::from_raw(width, height, rgba)?;
    let mut webp_bytes = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut webp_bytes);
    // Use lossy with quality
    encoder.encode(&img, width, height, image::ColorType::Rgba8).ok()?;
    Some(webp_bytes)
}
```

Update `capture_screen_gdi` to call `encode_webp` before returning:
```rust
// At end of capture_screen_gdi:
if lines > 0 {
    let webp_data = encode_webp(&buffer, width, height, 80).unwrap_or(buffer);
    Some(CapturedFrame {
        monitor_id,
        width,
        height,
        format: "webp",
        data: webp_data,
        timestamp_ns,
        bounds: BoundingRect::new(left, top, left + width as i32, top + height as i32),
    })
}
```

### P5.2 — Implement before/after screenshot pipeline

Add to `crates/capture-win/src/screenshot.rs` a `ScreenshotPipeline` struct:

```rust
pub struct ScreenshotPipeline {
    pub stabilization_delays_ms: Vec<u64>,
    pub diff_threshold: f32, // 0.005 = 0.5%
}

impl ScreenshotPipeline {
    /// Capture before-state screenshot and save to session dir.
    /// Returns path of saved file.
    pub fn capture_before(
        &self,
        monitor_id: u32,
        monitor_bounds: &BoundingRect,
        session_screenshots_dir: &Path,
        event_id: u64,
        timestamp_ns: u64,
    ) -> Option<PathBuf> {
        let frame = capture_screen_gdi(monitor_id, monitor_bounds.left, monitor_bounds.top,
            (monitor_bounds.right - monitor_bounds.left) as u32,
            (monitor_bounds.bottom - monitor_bounds.top) as u32,
            timestamp_ns)?;
        let path = session_screenshots_dir.join(format!("{:010}_before.webp", event_id));
        std::fs::write(&path, &frame.data).ok()?;
        Some(path)
    }

    /// Capture after-state with stabilization: wait for screen to stop changing.
    pub fn capture_after_stable(
        &self,
        monitor_id: u32,
        monitor_bounds: &BoundingRect,
        session_screenshots_dir: &Path,
        event_id: u64,
    ) -> Option<PathBuf> {
        let mut last_frame: Option<Vec<u8>> = None;
        for delay_ms in &self.stabilization_delays_ms {
            std::thread::sleep(Duration::from_millis(*delay_ms));
            let ts = crate::monotonic_ns();
            let frame = capture_screen_gdi(monitor_id, monitor_bounds.left, monitor_bounds.top,
                (monitor_bounds.right - monitor_bounds.left) as u32,
                (monitor_bounds.bottom - monitor_bounds.top) as u32, ts)?;
            let diff_ratio = if let Some(ref prev) = last_frame {
                pixel_diff_ratio(prev, &frame.data)
            } else {
                1.0 // First frame, not stable yet
            };
            last_frame = Some(frame.data.clone());
            if diff_ratio < self.diff_threshold {
                // Screen is stable
                let path = session_screenshots_dir.join(format!("{:010}_after.webp", event_id));
                std::fs::write(&path, &frame.data).ok()?;
                return Some(path);
            }
        }
        // Use last captured frame even if not fully stable
        if let Some(data) = last_frame {
            let path = session_screenshots_dir.join(format!("{:010}_after.webp", event_id));
            std::fs::write(&path, &data).ok();
            return Some(path);
        }
        None
    }
}

/// Compute pixel difference ratio between two BGRA/WebP frames.
/// Returns 0.0 (identical) to 1.0 (completely different).
fn pixel_diff_ratio(a: &[u8], b: &[u8]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let diff_count = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
    diff_count as f32 / a.len() as f32
}
```

### P5.3 — Video capture: GDI frame loop with timestamp (TODO stub for MF H.264)

In `crates/capture-win/src/video.rs`, implement a working GDI-based frame capture loop:

```rust
/// GDI-based frame capture loop. Captures frames at configured FPS.
/// TODO(MF-H264): Replace with Windows Media Foundation H.264 encoder for production quality.
pub struct VideoRecorder {
    config: VideoConfig,
    is_running: Arc<AtomicBool>,
    output_dir: PathBuf,
    start_monotonic_ns: u64,
}

impl VideoRecorder {
    pub fn new(config: VideoConfig, output_dir: PathBuf) -> Self {
        Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            output_dir,
            start_monotonic_ns: 0,
        }
    }

    /// Start capturing frames in a background thread.
    /// Frames are saved as individual WebP files for now.
    /// Returns a JoinHandle and the stop signal.
    pub fn start(&mut self) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
        let is_running = Arc::clone(&self.is_running);
        is_running.store(true, Ordering::SeqCst);
        self.start_monotonic_ns = crate::monotonic_ns();
        
        let fps = self.config.fps;
        let output_dir = self.output_dir.clone();
        let running = Arc::clone(&is_running);
        
        let handle = std::thread::spawn(move || {
            let frame_interval = Duration::from_millis(1000 / fps as u64);
            let mut frame_index = 0u64;
            
            while running.load(Ordering::Relaxed) {
                let ts_ns = crate::monotonic_ns();
                // TODO(MF-H264): Use WGC or Media Foundation for hardware-accelerated H.264
                // For now: capture primary monitor GDI frame
                #[cfg(windows)]
                if let Some(frame) = super::screenshot::native::capture_screen_gdi(0, 0, 0, 1920, 1080, ts_ns) {
                    let path = output_dir.join(format!("frame_{:08}_{}.webp", frame_index, ts_ns));
                    let _ = std::fs::write(&path, &frame.data);
                }
                
                frame_index += 1;
                std::thread::sleep(frame_interval);
            }
        });
        
        (handle, is_running)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
```

---

## Priority 6 — SUPERVISOR: Real Disk Stats + Windows Service

### P6.1 — Real disk stats with `GetDiskFreeSpaceExW`

In `apps/supervisor/src/main.rs`, replace mock disk stats (lines 58-60) with:

```rust
#[cfg(windows)]
fn get_disk_stats(path: &std::path::Path) -> (u64, u64) {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::Win32::Foundation::ULARGE_INTEGER;
    
    let path_wide = HSTRING::from(path.to_str().unwrap_or("C:\\"));
    let mut free_bytes_available = ULARGE_INTEGER::default();
    let mut total_bytes = ULARGE_INTEGER::default();
    let mut total_free_bytes = ULARGE_INTEGER::default();
    
    unsafe {
        let _ = GetDiskFreeSpaceExW(
            &path_wide,
            Some(&mut free_bytes_available),
            Some(&mut total_bytes),
            Some(&mut total_free_bytes),
        );
    }
    
    (unsafe { *total_bytes.QuadPart() as u64 }, unsafe { *free_bytes_available.QuadPart() as u64 })
}

#[cfg(not(windows))]
fn get_disk_stats(_path: &std::path::Path) -> (u64, u64) {
    // Fallback for non-Windows builds (tests)
    (500 * 1024 * 1024 * 1024, 200 * 1024 * 1024 * 1024)
}
```

Then in the disk watchdog loop:
```rust
let (total_bytes, available_bytes) = get_disk_stats(&spool_root);
let level = evaluate_disk_level(total_bytes, available_bytes, &watermark_config);
```

### P6.2 — Windows Service control handler

Create `apps/supervisor/src/service.rs` implementing proper Windows Service lifecycle:

```rust
/// Windows Service entry point.
/// Invoked by Service Control Manager when running as a service.
/// When run with --install-service flag instead, registers itself.

use windows_service::{
    define_windows_service,
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn run_as_service() -> windows_service::Result<()> {
    service_dispatcher::start("TrajectoryRecorder", ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<std::ffi::OsString>) {
    if let Err(e) = run_service() {
        tracing::error!("Service failed: {:?}", e);
    }
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    
    let event_handler = move |control_event| match control_event {
        ServiceControl::Stop => {
            RUNNING.store(false, Ordering::SeqCst);
            let _ = tx.send(());
            ServiceControlHandlerResult::NoError
        }
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    
    let status_handle = service_control_handler::register("TrajectoryRecorder", event_handler)?;
    
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;
    
    // Main supervisor logic runs here until RUNNING becomes false
    crate::run_supervisor_loop();
    
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;
    
    Ok(())
}
```

Modify `apps/supervisor/src/main.rs` to:
1. Check for `--install-service` flag: if present, register with SCM using `windows-service` install API.
2. Check for `--run-service` flag: if present, call `run_as_service()`.
3. Otherwise: run normally as a console process (for development).

Extract the main supervisor logic into `run_supervisor_loop()` function so it can be called from both service and console modes.

---

## Priority 7 — DESKTOP UI: Real Session Data

### P7.1 — Wire Tauri commands to real session data

In `apps/desktop-ui/src/main.rs`, replace hardcoded fixture timeline with Tauri commands that:

1. **`list_sessions` command**: Scan `spool/` directory for all session subdirs across all states. For each, read `manifest.json` and return metadata.

2. **`get_session_events` command**: Given a `session_id`, read `events.normalized.ndjson` line by line and return events array to frontend.

3. **`get_screenshot` command**: Given a `session_id` and `event_id`, return base64-encoded WebP bytes for display.

Minimal Tauri command implementations:
```rust
#[tauri::command]
async fn list_sessions(spool_root: String) -> Result<Vec<serde_json::Value>, String> {
    let spool_path = std::path::Path::new(&spool_root);
    let mut sessions = Vec::new();
    
    for state_dir in ["recording", "pending_upload", "uploading", "uploaded", "failed"] {
        let state_path = spool_path.join(state_dir);
        if !state_path.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(&state_path) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                            sessions.push(manifest);
                        }
                    }
                }
            }
        }
    }
    
    Ok(sessions)
}

#[tauri::command]
async fn get_session_events(session_path: String) -> Result<Vec<serde_json::Value>, String> {
    let ndjson_path = std::path::Path::new(&session_path).join("events.normalized.ndjson");
    let mut events = Vec::new();
    
    if let Ok(content) = std::fs::read_to_string(&ndjson_path) {
        for line in content.lines() {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                events.push(event);
            }
        }
    }
    
    Ok(events)
}
```

---

## Priority 8 — MANDATORY DOCUMENTATION

Create all missing required documentation files:

### `README.md` (repo root)
Create with: project description, architecture overview, quick start (build, run dev server with docker compose, run agent), Windows deployment instructions (install-service), and links to ARCHITECTURE.md, DATA_SCHEMA.md, SECURITY.md, DEVELOPMENT.md.

### `ARCHITECTURE.md`
Create from the existing PROJECT.md architecture section. Include: system topology diagram (ASCII art), all 5 binaries, all 20 crates, data flow (Collectors → Event Bus → Privacy → Correlator → Session → Spool → Upload → Server), IPC protocol, session directory structure, spool state machine diagram.

### `DATA_SCHEMA.md`
Create documenting all persisted schema:
- `RawEvent` NDJSON line format (from core-types)
- `CanonicalAction` NDJSON line format (from core-types)
- `manifest.json` fields
- Session directory layout
- Archive (`.tar.zst`) contents
- Object storage key structure
- PostgreSQL tables (machines, sessions, session_chunks, heartbeats)
- Schema versioning policy

### `SECURITY.md`
Document: privacy filtering rules, encryption (XChaCha20-Poly1305), key management (DPAPI), credential storage (never in source, env vars for server), what data is redacted, what is not captured, Windows secure desktop exclusion.

### `DEVELOPMENT.md`
Document: prerequisites (Rust 1.85+, cargo, Docker, Node.js), build commands (`cargo build --workspace`), running dev stack (`docker compose up -d` + server + agent), running tests (`cargo test --workspace`), installing as service (`trajectory-supervisor --install-service`), common troubleshooting.

### `Cargo.lock`
Run `cargo generate-lockfile` after all Cargo.toml changes are made to generate `Cargo.lock`. Ensure it is committed.

---

## Quality Rules for Implementation

1. **No `unwrap()` in production paths** — use `?`, `map_err`, or log-and-continue.
2. **No hardcoded secrets** — server reads all secrets from env vars.
3. **Every `unsafe` block must have a SAFETY comment** explaining the invariant.
4. **Windows API calls must handle INVALID_HANDLE_VALUE and error codes** — use `GetLastError()` on failure.
5. **No mock stubs with "TODO in production" comments in code paths that are supposed to be production** — either implement or note as explicitly out-of-scope with a filed tracking note.
6. **Each crate must have at least one test** for its primary functionality.
7. **After all changes, run**: describe what `cargo check --workspace` would show (since no cargo in PATH on target machine, verify logic manually).

## After Each Priority Group

Report:
```
## Phase X Complete
### Files Changed
### What was implemented
### Tests added/passing
### Known remaining limitations
### Next phase
```

Do NOT report "Done" without this structure.

## Final Deliverable

After all 8 priorities are complete:
1. All `use X-Y::` patterns fixed across entire workspace
2. All `RawEventPayload` references match actual core-types enum
3. All missing Cargo.toml deps added
4. Crash-safe GlobalEventIdAllocator in place
5. Session creates full directory structure + manifest.json + events.normalized.ndjson
6. Uploader does real HTTP chunk upload with retry and waits for SESSION_ACCEPTED
7. Server uses PostgreSQL + S3 + real JWT + real SHA-256 verification
8. Screenshot pipeline encodes WebP + captures before/after with stabilization
9. Video recorder has working GDI frame loop (TODO note for MF H.264)
10. Supervisor uses real GetDiskFreeSpaceExW + Windows Service scaffold
11. Desktop UI reads real session manifest.json and events.normalized.ndjson
12. All 5 mandatory docs created: README.md, ARCHITECTURE.md, DATA_SCHEMA.md, SECURITY.md, DEVELOPMENT.md
13. Cargo.lock generated
