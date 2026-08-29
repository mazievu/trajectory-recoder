# Trajectory Recorder: Continuous Windows Workflow & Multimodal Interaction Capture

[![Rust](https://img.shields.io/badge/Rust-Edition%202024%20(1.85+)-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20x64-blue.svg)](https://www.microsoft.com/windows)
[![Architecture](https://img.shields.io/badge/Architecture-Process%20Isolated%20Microservices-green.svg)](#system-topology--component-roles)
[![Security](https://img.shields.io/badge/Security-DPAPI%20%2B%20XChaCha20--Poly1305%20%2B%20Fail--Closed-red.svg)](SECURITY.md)
[![License](https://img.shields.io/badge/License-Proprietary-lightgrey.svg)](#)

---

## Overview

**Trajectory Recorder** is an enterprise-grade, continuous 24/7 desktop workflow and multimodal interaction capture system engineered natively in **Rust 2024** for Windows 10 and Windows 11. 

The system operates unobtrusively in the background to capture user interactions, semantic UI hierarchies, active window contexts, multi-monitor WebP visual state changes, and browser telemetry. Captured interactions are filtered in-memory for privacy, correlated into canonical actions, persisted locally using crash-resilient hourly partitions (NDJSON + SQLite WAL), encrypted with authenticated AEAD ciphers (`XChaCha20-Poly1305`), and securely uploaded to an enterprise ingestion cluster (`Axum` + `PostgreSQL 16+` + `S3/MinIO`).

### Core Design Highlights
- **Kernel-Edge Capture**: Non-blocking Win32 low-level hooks (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`) with sub-50µs dispatch latency.
- **Process Isolation**: Strict separation between Session 0 Windows Service supervisor, interactive user session capture agent, native messaging browser host, background uploader, and desktop UI.
- **Dual Timestamp Engine**: Simultaneous capture of RFC 3339 UTC wall-clock time and microsecond/nanosecond monotonic clocks (QPC) with local timezone offsets.
- **Fail-Closed Privacy Engine**: In-memory zero-allocation redaction across 3 tiers (password box suppression, regex filtering with Luhn algorithm validation, and Shannon entropy thresholding).
- **Hourly Partitioning & Spool FSM**: 6-stage crash-resilient local spool pipeline (`recording/` → `finalizing/` → `pending_upload/` → `uploading/` → `uploaded/` / `failed/`).
- **Resilient Upload Engine**: TAR.Zstd streaming compression, 64–256 MiB chunking with individual SHA-256 verification, and exponential backoff with jitter.

---

## System Topology & Component Roles

The system is decomposed into 6 primary binaries, 19 modular crates, and a Manifest V3 browser extension:

```
[ Session 0: Windows Service ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-supervisor.exe                                   │
  │ • Machine Registration & DPAPI Token Protection             │
  │ • Process Lifetime Management (Agent & Uploader)            │
  │ • 4-Tier Disk Pressure & Spool Health Watchdog              │
  │ • Startup Crash Recovery & Orphan Session Rescuer           │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ Named Pipe IPC (rmp-serde)
                                 ▼
[ Interactive User Session: Desktop Capture ]
  ┌─────────────────────────────────────────────────────────────┐
  │ trajectory-agent.exe                                        │
  │ ├─ input-win (WH_MOUSE_LL, WH_KEYBOARD_LL message pump)     │
  │ ├─ window-win (SetWinEventHook active window & monitor topo)│
  │ ├─ uia-win (Dedicated STA COM UIAutomation tree walker)    │
  │ ├─ capture-win (WGC/GDI WebP screenshots & Video loop)      │
  │ ├─ clipboard-win & file-events-win (Clipboard & Dir watch)  │
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
  │ Stdio 4-byte prefix bridge  │   │       finalizing/         │
  └──────────────▲──────────────┘   │       pending_upload/     │
                 │ Stdio JSON       │       uploading/          │
  ┌──────────────┴──────────────┐   │       uploaded/           │
  │ Chrome / Edge Extension     │   │       failed/             │
  │ (Manifest V3 DOM & Events)  │   └─────────────┬─────────────┘
  └─────────────────────────────┘                 │
                                                  ▼
[ Background Upload Worker ]        ┌───────────────────────────┐
                                    │ trajectory-uploader.exe   │
                                    │ • TAR + Zstd Compression  │
                                    │ • XChaCha20-Poly1305 Enc  │
                                    │ • 64-256 MiB Chunking     │
                                    │ • Resumable HTTP Client   │
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
  │ trajectory-tray.exe (Tauri 2 + React + TypeScript)          │
  │ • System Tray Control & Status Indicators                   │
  │ • Step-by-Step Trajectory Viewer & Visual Diffs             │
  │ • DOM / UIA Element Hierarchy Inspector                     │
  └─────────────────────────────────────────────────────────────┘
```

### Component Details

| Executable / Binary | Execution Scope | Description |
|---|---|---|
| **`trajectory-supervisor.exe`** | Session 0 (SYSTEM) | Windows Service responsible for machine enrollment, DPAPI credential protection, agent process monitoring, 4-tier disk watchdog, and startup crash recovery. |
| **`trajectory-agent.exe`** | Interactive Session | Desktop capture process running in the active user session. Houses Win32 hooks, UIA engine, privacy redaction, action correlator, and SQLite/NDJSON persistence. |
| **`trajectory-browser-host.exe`**| Interactive Session | Chromium Native Messaging Host bridging browser extension stdio JSON streams to the agent via Named Pipes. |
| **`trajectory-uploader.exe`** | Background Worker | Asynchronous spool processor. Packages finalized sessions with TAR + Zstandard, encrypts chunks with XChaCha20-Poly1305, and uploads to the ingestion server. |
| **`trajectory-server`** | Cloud / Server | Distributed Axum HTTP ingestion server with PostgreSQL relational metadata storage and S3/MinIO object store chunk verification. |
| **`trajectory-tray.exe`** | Interactive Session | Desktop tray application providing capture toggles, status monitoring, and an interactive trajectory viewer. |

---

## Quick Start Guide

### 1. Prerequisites
- **Operating System**: Windows 10 (1809+) or Windows 11 x64.
- **Rust Toolchain**: Rust 2024 Edition (`1.85.0` or higher) with `x86_64-pc-windows-msvc` target.
- **C++ Build Tools**: Visual Studio 2022 C++ Build Tools (with Windows 10/11 SDK).
- **Backend Infrastructure** (for server and local testing):
  - Docker & Docker Compose (for PostgreSQL 16+ and MinIO).
  - Alternatively, standalone PostgreSQL and S3-compatible storage.

### 2. Configuration & Environment Setup
Clone the repository and prepare the development environment:

```bash
# Clone repository
git clone https://github.com/company/trajectory-recorder.git
cd trajectory-recorder

# Configure local development environment for server
cp server/.env.example server/.env
```

Start the local PostgreSQL and MinIO infrastructure:
```bash
docker compose -f server/docker-compose.yml up -d
```

### 3. Build Commands
Build all workspace packages in release mode:

```bash
# Verify compilation across all workspace crates and binaries
cargo check --workspace

# Build all production binaries in release mode
cargo build --workspace --release
```

Compiled binaries will be located in `target/release/`:
- `trajectory-supervisor.exe`
- `trajectory-agent.exe`
- `trajectory-browser-host.exe`
- `trajectory-uploader.exe`
- `trajectory-server.exe`
- `trajectory-tray.exe`
- `trajectory-harness.exe`

### 4. Running Locally

#### Step 4.1: Launch Ingestion Server
```bash
# Start ingestion server on http://localhost:8080
cargo run --bin trajectory-server
```

#### Step 4.2: Start Agent or Supervisor
```bash
# Run supervisor in console development mode
cargo run --bin trajectory-supervisor

# Alternatively, run the capture agent directly in your user session
cargo run --bin trajectory-agent
```

#### Step 4.3: Start Uploader Worker
```bash
# Start background chunk uploader
cargo run --bin trajectory-uploader
```

### 5. Running Tests & Verification Suites
The project includes a comprehensive 5-tier test suite:

```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run Tier 1 Feature Coverage Tests
cargo test -p tier1-feature -- --nocapture

# Run E2E Test Runner Verification Modules
cargo test -p e2e-runner -- --nocapture

# Launch the interactive UI Test Fixture application
cargo run -p harness-app -- --mode interactive
```

---

## Production Acceptance Criteria Compliance (AC 1 – AC 40)

The Trajectory Recorder codebase satisfies all 40 Production Acceptance Criteria defined in the Master Implementation Specification:

| AC # | Acceptance Criterion | Implementation Architecture & Crates | Status |
|---|---|---|---|
| **AC 1** | **Core Data Schemas** | `crates/core-types`: `GlobalEventId`, `SessionId`, `RawEvent` (10 payload types), `CanonicalAction` (39 action types), `DualTimestamp`, `TargetMetadata`. | **COMPLIANT** |
| **AC 2** | **Low-Level Mouse Hook** | `crates/input-win`: Non-blocking `WH_MOUSE_LL` hook pump (<50µs dispatch), physical/normalized coordinates, wheel delta, monitor mapping. | **COMPLIANT** |
| **AC 3** | **Low-Level Keyboard Hook** | `crates/input-win`: `WH_KEYBOARD_LL` hook, scan codes, virtual keys, modifier state tracking (`Ctrl`, `Alt`, `Shift`, `Win`, `Caps`, `Num`). | **COMPLIANT** |
| **AC 4** | **Active Window & Monitor** | `crates/window-win`: `SetWinEventHook` foreground listener, HWND, process name/PID, title, DPI, monitor topology. | **COMPLIANT** |
| **AC 5** | **COM STA UIA Engine** | `crates/uia-win`: Dedicated STA COM thread, `IUIAutomation` element inspection, 100ms timeout guard, BoundingBox. | **COMPLIANT** |
| **AC 6** | **Ancestor Hierarchy Walk** | `crates/uia-win`: 3-level parent hierarchy walk (Parent, Grandparent, Great-Grandparent), FrameworkId (`WPF`, `WinForms`, `Electron`, `Win32`). | **COMPLIANT** |
| **AC 7** | **Typing Burst Aggregator** | `crates/correlator`: 500ms debounce grouping into `TYPE_TEXT`, character count, backspace/delete tracking, enter detection. | **COMPLIANT** |
| **AC 8** | **Scroll Gesture Aggregator**| `crates/correlator`: 300ms debounce grouping into `SCROLL`, delta X/Y accumulation, container targeting. | **COMPLIANT** |
| **AC 9** | **Drag-and-Drop Machine** | `crates/correlator`: Mouse down → threshold move (>5px) → mouse up state machine into `DRAG_DROP` with distance calculation. | **COMPLIANT** |
| **AC 10** | **Privacy Engine Redaction** | `crates/privacy`: 3-tier in-memory engine: password box exclusion, regex (SSN, CC Luhn, API keys, JWT), Shannon entropy ($H > 4.5$). | **COMPLIANT** |
| **AC 11** | **Fail-Closed Masking** | `crates/privacy`: In-memory redaction before disk/IPC, `[PASSWORD_REDACTED]`, `[SSN_REDACTED]`, `[CREDIT_CARD_REDACTED]`, `[REDACTED]`. | **COMPLIANT** |
| **AC 12** | **Multi-Monitor Screenshots** | `crates/capture-win`: Multi-monitor GDI/WGC screen capture with WebP lossless/lossy compression. | **COMPLIANT** |
| **AC 13** | **State Change Visual Diffs** | `crates/capture-win`: Before/after screenshots with multi-step stabilization delays (200ms, 500ms, 1000ms) until diff < 0.5%. | **COMPLIANT** |
| **AC 14** | **Video Fragment Pipeline** | `crates/capture-win`: Continuous video frame capture loop with high-resolution monotonic timestamps. | **COMPLIANT** |
| **AC 15** | **Manifest V3 Extension** | `browser-extension`: Chrome/Edge MV3 companion extension with background service worker and content script. | **COMPLIANT** |
| **AC 16** | **DOM Selector Engine** | `browser-extension/selector.ts`: Robust selector resolution (ID, tag, role, visible text, ARIA label, CSS, XPath). | **COMPLIANT** |
| **AC 17** | **MutationObserver Engine** | `browser-extension`: DOM mutation listener tracking modals, alerts, toast notifications, and SPA transitions. | **COMPLIANT** |
| **AC 18** | **Native Messaging Host** | `apps/browser-host`: Length-prefixed stdio JSON bridge to Named Pipe (`\\.\pipe\trajectory-browser-host`). | **COMPLIANT** |
| **AC 19** | **Global Session ID Gen** | `crates/core-types`, `crates/session`: Monotonic Session ID `{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}` & crash-safe `GlobalEventIdAllocator`. | **COMPLIANT** |
| **AC 20** | **Seamless Hourly Rotation** | `crates/session`: Atomic session rotation on clock-hour boundaries with zero capture gaps and synchronized flushes. | **COMPLIANT** |
| **AC 21** | **SQLite WAL Persistence** | `crates/session`: Embedded SQLite database (`session.db`) with WAL mode, 7 relational tables, and query indexes. | **COMPLIANT** |
| **AC 22** | **Startup Crash Recovery** | `crates/session`, `apps/supervisor`: Scans `spool/recording/`, truncates corrupted NDJSON tail (<2s loss), marks `RECOVERED`. | **COMPLIANT** |
| **AC 23** | **6-Stage Spool Machine** | `crates/spool`: Atomic directory transitions: `recording` → `finalizing` → `pending_upload` → `uploading` → `uploaded` / `failed`. | **COMPLIANT** |
| **AC 24** | **TAR + Zstd Packaging** | `crates/archive`: Streaming TAR packaging with multi-threaded Zstandard compression and manifest generation. | **COMPLIANT** |
| **AC 25** | **XChaCha20-Poly1305 AEAD** | `crates/crypto`: 256-bit authenticated encryption with random 24-byte nonces, AAD binding, and SHA-256 integrity verification. | **COMPLIANT** |
| **AC 26** | **Chunking Engine** | `crates/archive`: Fixed-size chunking (64–256 MiB) with per-chunk SHA-256 calculation and manifest generation. | **COMPLIANT** |
| **AC 27** | **Resumable Upload Client** | `crates/upload-client`, `apps/uploader`: HTTP client with jittered exponential backoff, SHA-256 headers, and retry logic. | **COMPLIANT** |
| **AC 28** | **Axum Ingestion Server** | `apps/server`: High-throughput async REST server (`/api/v1/machines/*`, `/api/v1/sessions/*`). | **COMPLIANT** |
| **AC 29** | **PostgreSQL Schema & Mig** | `server/migrations`: Relational migrations for `machines`, `sessions`, `session_chunks`, and `machine_heartbeats`. | **COMPLIANT** |
| **AC 30** | **S3 / MinIO Chunk Storage** | `apps/server`: Direct streaming of encrypted chunks to object storage with partitioned storage key hierarchy. | **COMPLIANT** |
| **AC 31** | **Chunk Verification & Rec**| `apps/server`: On-the-fly SHA-256 chunk validation, multipart reassembly verification, and `SESSION_ACCEPTED` issuance. | **COMPLIANT** |
| **AC 32** | **Tauri 2 Tray & Control** | `apps/desktop-ui`: Desktop system tray icon, capture status indicator, start/pause/stop controls. | **COMPLIANT** |
| **AC 33** | **Interactive Timeline** | `apps/desktop-ui`: Step-by-step scrubbable action timeline loaded directly from `events.normalized.ndjson`. | **COMPLIANT** |
| **AC 34** | **Before/After Visual Diff** | `apps/desktop-ui`: Side-by-side screenshot viewer with target bounding boxes and perceptual diff overlays. | **COMPLIANT** |
| **AC 35** | **DOM & UIA Tree Inspector** | `apps/desktop-ui`: Semantic element hierarchy inspector displaying AutomationId, class, role, and CSS selectors. | **COMPLIANT** |
| **AC 36** | **Multi-Faceted Search** | `apps/desktop-ui`: Multi-attribute session filtering by app, user, machine, action type, date range, and target text. | **COMPLIANT** |
| **AC 37** | **Test Harness Application** | `tests/harness-app`: Dedicated Win32 / UIA test fixture (`trajectory-harness.exe`) with 12 standardized control types. | **COMPLIANT** |
| **AC 38** | **4-Tier Disk Protection** | `crates/spool`, `apps/supervisor`: Disk watermark monitoring (<70%, 70-85%, 85-92%, >92%) with automatic retention purging. | **COMPLIANT** |
| **AC 39** | **Power-Off & Kill Recovery** | `crates/session`, `tests/e2e-runner`: Resilience against abrupt `taskkill /F`, corrupt WAL auto-truncation, and resumable uploads. | **COMPLIANT** |
| **AC 40** | **30-Min Continuous E2E** | `tests/e2e-runner`, `tests/tier4-workload`: Verified 30-minute cross-application workflow reconstruction (19 attributes). | **COMPLIANT** |

---

## Documentation Navigation

- 📐 **[ARCHITECTURE.md](ARCHITECTURE.md)**: Deep dive into architectural design, process isolation, IPC protocols, Spool state machine, concurrency models, and security architecture.
- 🗄️ **[DATA_SCHEMA.md](DATA_SCHEMA.md)**: Exhaustive schema documentation for Raw Events, Canonical Actions, SQLite WAL database, PostgreSQL tables, manifests, and NDJSON streaming formats.
- 🔒 **[SECURITY.md](SECURITY.md)**: Comprehensive threat model, 3-tier privacy engine, DPAPI key protection, SDDL pipe security, and encryption at rest/in transit.
- 🛠️ **[DEVELOPMENT.md](DEVELOPMENT.md)**: Developer setup, toolchain requirements, building, testing with `trajectory-harness.exe`, migrations, and code quality standards.
"# trajectory-recoder" 
