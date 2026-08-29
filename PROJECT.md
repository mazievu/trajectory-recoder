# Project: Trajectory Recorder (Windows Rust 2024 Production System)

## Architecture

The **Trajectory Recorder** is a high-performance, enterprise-grade, continuous 24/7 desktop workflow and multimodal interaction capture system on Windows. It combines kernel-edge Win32 hooks, UI Automation (UIA), Windows Graphics Capture (WGC), Media Foundation video encoding, Manifest V3 browser telemetry, privacy filtering, hourly partition persistence (NDJSON + SQLite WAL), encrypted chunked spooling (XChaCha20-Poly1305 + TAR.Zstd), and a distributed Axum + PostgreSQL 16 + S3/MinIO Ingestion Server and Tauri 2 Desktop UI.

### System Topology & Process Boundaries
```
[ Session 0: Windows Service ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-supervisor.exe                                   │
  │ - Machine Registration & DPAPI Token Protection             │
  │ - Process Lifetime Management (Agent & Uploader)            │
  │ - Disk Pressure & Spool Health Watchdog                     │
  │ - Crash Recovery Scanner & Orphan Session Rescuer           │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ Named Pipe IPC (rmp-serde)
                                 ▼
[ Interactive User Session: Desktop Capture ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-agent.exe                                        │
  │ ├─ input-win (WH_MOUSE_LL, WH_KEYBOARD_LL message pump)     │
  │ ├─ window-win (SetWinEventHook active window & monitor topo)│
  │ ├─ uia-win (COM STA/MTA UIAutomation worker & tree walker)  │
  │ ├─ capture-win (WGC/DDA WebP screenshot & H.264 video MFT)  │
  │ ├─ clipboard-win & file-events-win (Clipboard & File Dialog)│
  │ ├─ privacy (In-memory regex, entropy, password redaction)   │
  │ ├─ correlator (Action builder & burst grouping)             │
  │ └─ session (Hourly partition router, NDJSON & SQLite WAL)   │
  └──────────────────▲────────────────────────┬─────────────────┘
                     │                        │
       Named Pipe    │                        │ Spool Directory
       IPC           │                        │ State Transitions
  ┌──────────────────┴──────────┐             ▼
  │ trajectory-browser-host.exe │   ┌───────────────────────────┐
  │ (Native Messaging Bridge)   │   │ spool/recording/          │
  └──────────────▲──────────────┘   │       finalizing/         │
                 │ Stdio JSON       │       pending_upload/     │
  ┌──────────────┴──────────────┐   │       uploading/          │
  │ Chrome / Edge Extension     │   │       uploaded/           │
  │ (Manifest V3 DOM & Events)  │   │       failed/             │
  └─────────────────────────────┘   └─────────────┬─────────────┘
                                                  │
                                                  ▼
[ Background Upload Worker ]        ┌───────────────────────────┐
                                    │ trajectory-uploader.exe   │
                                    │ - TAR + Zstd Compression  │
                                    │ - XChaCha20-Poly1305 Enc  │
                                    │ - 64-256 MiB Chunking     │
                                    │ - Resumable HTTP Client   │
                                    └─────────────┬─────────────┘
                                                  │ HTTPS REST
                                                  ▼
[ Cloud / Enterprise Ingestion Server ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-server (Axum + Tokio)                            │
  │ ├─ REST Ingestion APIs (/api/v1/machines/*, /sessions/*)    │
  │ ├─ PostgreSQL 16+ (Metadata, Sessions, Chunks, Metrics)     │
  │ └─ S3 / MinIO Object Storage (Encrypted Chunk Store)        │
  └─────────────────────────────────────────────────────────────┘

[ Desktop UI & Trajectory Viewer ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-tray.exe (Tauri 2 + React 19 + TypeScript)       │
  │ - System Tray Control & Status Indicators                   │
  │ - Interactive Step-by-Step Trajectory Viewer & Visual Diffs │
  │ - DOM / UIA Element Hierarchy Inspector                     │
  └─────────────────────────────────────────────────────────────┘
```

---

## Feature Inventory

Every feature and technical requirement identified across `spec.md`, `producttechnical requirement.md`, and Acceptance Criteria AC 1–40 is indexed below with its owning Milestone.

| # | Feature | Description | Milestone | Source |
|---|---|---|---|---|
| 1 | Cargo Workspace Setup | Edition 2024, 19 crates, 5 binaries, strict dependency graph, MSVC flags | M1 | spec §4, req §4 |
| 2 | Dual Timestamp Engine | Wall-clock UTC (RFC3339) + Monotonic nanoseconds (QPC/Instant), timezone offset | M1 | spec §8, req §8 |
| 3 | Core Data Schemas | `GlobalEventId`, `SessionId`, `RawEvent`, `CanonicalAction` (36+ types), `TargetMetadata` | M1 | spec §8-11, AC 1 |
| 4 | Hierarchical Config | TOML/JSON config, machine identity, capture policies, live reload | M1 | spec §6, req §6 |
| 5 | Windows Named Pipe IPC | Duplex Named Pipe with `rmp-serde` MessagePack framing & security SDDL | M1 | spec §5, req §5 |
| 6 | Windows DPAPI Security | Device secret encryption via `CryptProtectData` (`CRYPTPROTECT_LOCAL_MACHINE`) | M1 | spec §42, req §31 |
| 7 | Low-Level Mouse Hook | `WH_MOUSE_LL` hook pump, non-blocking queue (<50µs), physical/normalized coords | M2 | spec §12, AC 2 |
| 8 | Low-Level Keyboard Hook | `WH_KEYBOARD_LL` hook, scan codes, virtual keys, modifier state tracking | M2 | spec §13, AC 3 |
| 9 | Active Window & Monitor | `SetWinEventHook` foreground tracker, HWND, process exe/title, monitor topology | M2 | spec §14, AC 4 |
| 10 | Bounded Event Bus | Multi-producer bounded channel with priority shed (P0 Input -> P4 Video) | M2 | spec §15, req §13 |
| 11 | Crash-Resilient NDJSON | Append-only raw event writer (`events.raw.ndjson`), buffered <2s sync flush | M2 | spec §37, AC 31 |
| 12 | Clipboard Listener | `AddClipboardFormatListener` format tracking, byte length, SHA-256 digest | M2 | spec §16, req §15 |
| 13 | File Telemetry & Dialogs | `ReadDirectoryChangesW` user folder tracker, Common File Dialog hook | M2 | spec §17, req §16 |
| 14 | COM STA UIA Engine | Dedicated STA COM worker, ControlType, AutomationId, name, rect, 100ms timeout | M3 | spec §20, AC 5 |
| 15 | Ancestor Hierarchy Walk | 3-level parent tree capture, framework ID (`WPF`, `WinForms`, `Electron`, `Win32`) | M3 | spec §21, AC 6 |
| 16 | Typing Burst Aggregator | 500ms debounce grouping into `TYPE_TEXT`, character count, backspace/delete | M3 | spec §23, AC 7 |
| 17 | Scroll Gesture Aggregator| 300ms debounce grouping into `SCROLL`, delta X/Y, momentum detection | M3 | spec §24, AC 8 |
| 18 | Drag-and-Drop Machine | Mouse down -> threshold move -> mouse up state machine into `DRAG_DROP` | M3 | spec §25, AC 9 |
| 19 | Privacy Engine Redaction | Password box exclusion, regex (SSN, credit card Luhn, API keys), Shannon entropy | M3 | spec §26-28, AC 10 |
| 20 | Fail-Closed Masking | In-memory redaction before disk/IPC, `[REDACTED]` / `[UNOBSERVED_TEXT]` | M3 | spec §29, AC 11 |
| 21 | Multi-Monitor Screenshots | Windows Graphics Capture (WGC) + DDA fallback, multi-screen WebP encoding | M4 | spec §30, AC 12 |
| 22 | State Change Visual Diffs | Perceptual diff capture at +200ms, +500ms, +1000ms until <0.5% pixel change | M4 | spec §31, AC 13 |
| 23 | Video Fragment Pipeline | Continuous Media Foundation H.264 (10 FPS, 1500kbps, 2.0s GOP) + timestamp index | M4 | spec §32, AC 14 |
| 24 | Manifest V3 Extension | Chrome/Edge MV3 extension (`background.js`, `content.js`), DOM event listeners | M5 | spec §18, AC 15 |
| 25 | DOM Selector Engine | Robust selector resolution: tag, role, visible text, ARIA, CSS, DOM path, XPath | M5 | spec §18, AC 16 |
| 26 | MutationObserver Engine | DOM mutation listener for modals, alerts, toast notifications, SPA transitions | M5 | spec §19, AC 17 |
| 27 | Native Messaging Host | `trajectory-browser-host.exe` stdio 4-byte length prefix <-> Named Pipe bridge | M5 | spec §4.5, AC 18 |
| 28 | Global Session ID Gen | Monotonic session ID `{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}` & crash counter | M6 | spec §33, AC 19 |
| 29 | Seamless Hourly Rotation | Atomic session handoff on clock-hour boundaries (08:00->09:00) with zero capture gap | M6 | spec §34, AC 20 |
| 30 | SQLite WAL Persistence | Embedded SQLite WAL schema (7 tables, triggers, indexes) for action/evidence index | M6 | spec §35, AC 21 |
| 31 | Startup Crash Recovery | Scan `spool/recording/`, truncate corrupt NDJSON tail (<2s), mark `RECOVERED` | M6 | spec §38, AC 22 |
| 32 | 6-Stage Spool Machine | Atomic dir renames: `recording`->`finalizing`->`pending_upload`->`uploading`->`uploaded`| M7 | spec §39, AC 23 |
| 33 | TAR + Zstd Packaging | Multi-threaded streaming Zstd compression of finalized sessions with manifest | M7 | spec §40, AC 24 |
| 34 | XChaCha20-Poly1305 AEAD | Authenticated 256-bit encryption with random 24-byte nonces & SHA-256 digests | M7 | spec §41, AC 25 |
| 35 | Chunking Engine | Resumable chunking (64–256 MiB chunks) with individual SHA-256 chunk manifests | M7 | spec §41, AC 26 |
| 36 | Resumable Upload Client | HTTP upload client with jittered exponential backoff & bandwidth throttling | M7 | spec §46, AC 27 |
| 37 | Axum Ingestion Server | High-throughput async REST server (`/api/v1/machines/*`, `/api/v1/sessions/*`) | M8 | spec §43, AC 28 |
| 38 | PostgreSQL Schema & Mig | Relational schema with migrations for machines, sessions, chunks, heartbeats | M8 | spec §44, AC 29 |
| 39 | S3 / MinIO Chunk Storage | Object storage integration for encrypted session chunks and archive blobs | M8 | spec §45, AC 30 |
| 40 | Chunk Verification & Rec | On-the-fly SHA-256 chunk validation, multipart reassembly & session acceptance | M8 | spec §45, AC 31 |
| 41 | Tauri 2 Tray & Control | Desktop tray icon, capture status, start/pause/stop toggle, disk indicators | M9 | spec §53, AC 32 |
| 42 | Interactive Timeline | Scrubbable timeline card stream with canonical action metadata & app icons | M9 | spec §54, AC 33 |
| 43 | Before/After Visual Diff | Side-by-side screenshot viewer with vector bounding boxes & visual diff overlays | M9 | spec §54, AC 34 |
| 44 | DOM & UIA Tree Inspector | Interactive tree viewer for element hierarchy, properties, and CSS selectors | M9 | spec §54, AC 35 |
| 45 | Multi-Faceted Search | Filter by app, user, machine, action type, date range, error, target text | M9 | spec §54, AC 36 |
| 46 | Test Harness Application | Standardized Win32/Slint test fixture (`trajectory-harness.exe`) for automation | M10 | spec §55, AC 37 |
| 47 | 4-Tier Disk Protection | Disk watermark monitoring (<70%, 70-85%, 85-92%, >92%) & emergency throttle | M10 | spec §48, AC 38 |
| 48 | Power-Off & Kill Recovery | Verification against abrupt `taskkill /F`, corrupt WAL, and partial chunks | M10 | spec §50, AC 39 |
| 49 | 30-Min Continuous E2E | 30-min soak test crossing hourly boundary (Chrome->Excel->Explorer->Photoshop) | M10 | spec §52, AC 40 |
| 50 | Opaque-Box E2E Test Suite| Tiers 1-4 comprehensive requirement-driven test cases (5×N per tier) | E2E | test-infra |

---

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| E2E | E2E Testing Track | Requirement-driven opaque-box test runner, mock environments, Tiers 1–4 suites | none | DONE |
| M1 | Phase 1 — Foundation | Cargo workspace (Edition 2024), `core-types`, dual timestamps, config, Named Pipe IPC, DPAPI | none | DONE |
| M2 | Phase 2 — Capture Core | Win32 hooks (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`), window tracking, Event Bus, NDJSON writer, clipboard, file telemetry | M1 | DONE |
| M3 | Phase 3 — Semantic Capture | UIAutomation COM worker, 3-level ancestor tree, typing/scroll burst grouping, drag&drop, Privacy Engine | M2 | DONE |
| M4 | Phase 4 — State Evidence | Multi-monitor WebP screenshot capture, perceptual diff stabilization, Media Foundation H.264 video pipeline | M2, M3 | DONE |
| M5 | Phase 5 — Browser Companion | Chrome/Edge MV3 extension, DOM selector engine, MutationObserver, Native Messaging Host bridge | M1, M2 | DONE |
| M6 | Phase 6 — Session Engine & Persistence | Global monotonic ID, hourly session rotation, SQLite WAL persistence, startup crash recovery | M2, M3, M4 | DONE |
| M7 | Phase 7 — Upload Pipeline & Spool | 6-stage Spool state machine, TAR+Zstd packaging, XChaCha20-Poly1305 encryption, chunking, resumable uploader | M6 | DONE |
| M8 | Phase 8 — Ingestion Server & API | Axum REST APIs, PostgreSQL migrations, S3/MinIO chunk storage, chunk verification & reassembly worker | M1, M7 | DONE |
| M9 | Phase 9 — Desktop UI & Viewer | Tauri 2 + React + TypeScript desktop tray app, timeline card stream, visual diffs, DOM/UIA inspector | M1, M6 | DONE |
| M10| Phase 10 — Hardening & Full Verification | Test harness app, 4-tier disk protection, recovery tests, 100% E2E test pass (Tiers 1-4), Tier 5 adversarial hardening | M1–M9, E2E | DONE |

---

## Interface Contracts

### 1. `core-types` ↔ All Capture & Persistence Crates
```rust
// Dual Timestamp representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualTimestamp {
    pub wall_time_utc: chrono::DateTime<chrono::Utc>,
    pub monotonic_ns: u64,
    pub timezone_offset_secs: i32,
}

// Global Unique Event Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalEventId(pub u64);

// Canonical Action Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAction {
    pub schema: String, // "gtf.trajectory"
    pub schema_version: String, // "1.0"
    pub global_event_id: GlobalEventId,
    pub session_id: String,
    pub session_event_id: u64,
    pub timestamp: DualTimestamp,
    pub action_type: ActionType,
    pub confidence: f32, // 0.0 .. 1.0
    pub target: TargetMetadata,
    pub context: ContextMetadata,
    pub state_evidence: Option<StateEvidence>,
    pub duration_ms: Option<u64>,
}
```

### 2. `ipc` ↔ `trajectory-agent` & `trajectory-supervisor`
- Protocol: Length-prefixed MessagePack (`rmp-serde`) over duplex Windows Named Pipe (`\\.\pipe\trajectory-agent-ipc` and `\\.\pipe\trajectory-browser-host`).
- Framing: 4-byte big-endian payload length header followed by binary payload.
- Message Types: `RegisterAgent`, `Heartbeat`, `SessionBoundarySignal`, `DiskWatermarkAlert`, `ConfigUpdate`, `BrowserDomEvent`.

### 3. `session` ↔ `spool`
- Atomic folder state transitions:
  `spool/recording/{session_id}` → `spool/finalizing/{session_id}` → `spool/pending_upload/{session_id}` → `spool/uploading/{session_id}` → `spool/uploaded/{session_id}` (or `spool/failed/{session_id}`).
- SQLite WAL Database schema: `session_meta`, `raw_events`, `canonical_actions`, `screenshots`, `video_segments`, `annotations`, `id_allocator`.

### 4. `trajectory-uploader` ↔ `trajectory-server` (Axum REST API)
- `POST /api/v1/machines/register` -> `{ machine_id, token }`
- `POST /api/v1/machines/heartbeat` -> `{ status: "ok" }`
- `POST /api/v1/sessions` -> `{ session_id, upload_url, expected_chunks }`
- `PUT /api/v1/sessions/{session_id}/chunks/{chunk_index}` (with `X-Chunk-SHA256` header) -> `{ chunk_index, status: "stored" }`
- `GET /api/v1/sessions/{session_id}/upload-status` -> `{ uploaded_chunks: [0, 1, 2], missing_chunks: [3] }`
- `POST /api/v1/sessions/{session_id}/complete` -> `{ status: "accepted", archive_sha256_verified: true }`

---

## Code Layout

```
trajectory-recorder/
├── Cargo.toml                       # Workspace manifest (Edition 2024, resolver = "3")
├── Cargo.lock
├── rust-toolchain.toml              # channel = "stable", edition = "2024"
├── apps/
│   ├── supervisor/                  # trajectory-supervisor.exe (Windows Service Session 0)
│   ├── capture-agent/               # trajectory-agent.exe (Interactive Desktop Agent)
│   ├── uploader/                    # trajectory-uploader.exe (Encrypted Chunk Uploader)
│   ├── browser-host/                # trajectory-browser-host.exe (Native Messaging Host)
│   ├── server/                      # trajectory-server (Axum Ingestion Server)
│   └── desktop-ui/                  # trajectory-tray.exe (Tauri 2 + React UI)
├── crates/
│   ├── core-types/                  # Foundation types, timestamps, schemas (pure Rust)
│   ├── config/                      # TOML config manager & policy schemas
│   ├── ipc/                         # Named Pipe IPC with rmp-serde framing
│   ├── event-bus/                   # High-throughput bounded MPMC bus with priority drop
│   ├── input-win/                   # Win32 WH_MOUSE_LL & WH_KEYBOARD_LL hooks
│   ├── window-win/                  # SetWinEventHook active window & monitor tracker
│   ├── uia-win/                     # COM STA/MTA UIAutomation worker & tree walker
│   ├── capture-win/                 # WGC/DDA WebP capture & MediaFoundation H.264 video
│   ├── browser-events/              # Extension DOM schema & bridge models
│   ├── clipboard-win/               # AddClipboardFormatListener & hash digest
│   ├── file-events-win/             # ReadDirectoryChangesW & Common File Dialogs
│   ├── privacy/                     # Regex, entropy, password redaction filter
│   ├── correlator/                  # Action builder, burst grouping, confidence scoring
│   ├── session/                     # Session router, hourly rotation, SQLite WAL, NDJSON
│   ├── spool/                       # Spool state machine, startup scanner, 4-tier disk
│   ├── archive/                     # Streaming TAR + Zstandard packaging
│   ├── crypto/                      # DPAPI & XChaCha20-Poly1305 encryption
│   ├── upload-client/               # Resumable HTTP chunked uploader client
│   ├── diagnostics/                 # Tracing subscriber, metrics, structured logs
│   └── test-support/                # Mock inputs, fake UIA trees, synthetic events
├── browser-extension/               # Chrome/Edge Manifest V3 Extension (TypeScript)
│   ├── manifest.json
│   ├── background.ts
│   ├── content.ts
│   └── selector.ts
├── server/                          # Database migrations & deployment configs
│   ├── migrations/
│   │   ├── 0001_initial_schema.sql
│   │   └── 0002_indexes.sql
│   └── docker-compose.yml
└── tests/                           # E2E and System Integration Suites
    ├── harness-app/                 # trajectory-harness.exe (Windows UI test fixture)
    ├── e2e-runner/                  # Requirement-driven E2E opaque-box test runner
    ├── tier1-feature/               # Tier 1 Feature Coverage test cases
    ├── tier2-boundary/              # Tier 2 Boundary & Corner test cases
    ├── tier3-pairwise/              # Tier 3 Cross-Feature Combination test cases
    └── tier4-workload/              # Tier 4 Real-World Application test scenarios
```
