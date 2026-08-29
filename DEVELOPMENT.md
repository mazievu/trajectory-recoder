# Trajectory Recorder: Developer Guide & Engineering Standards

**Target Rust Version**: Rust 2024 Edition (`1.85.0+`)  
**Target Platform**: Windows 10 / 11 x64 (`x86_64-pc-windows-msvc`)  
**Document Classification**: Engineering Guide & Quality Standards  

---

## Table of Contents
1. [Developer Setup & Toolchain Configuration](#1-developer-setup--toolchain-configuration)
2. [Local Infrastructure: PostgreSQL & MinIO](#2-local-infrastructure-postgresql--minio)
3. [Building & Running Components](#3-building--running-components)
4. [Testing Framework & Test Fixture (`trajectory-harness.exe`)](#4-testing-framework--test-fixture-trajectory-harnessexe)
5. [Database Migrations](#5-database-migrations)
6. [Production Quality Standards & Coding Rules](#6-production-quality-standards--coding-rules)
7. [Troubleshooting & Diagnostics](#7-troubleshooting--diagnostics)

---

## 1. Developer Setup & Toolchain Configuration

### 1.1 Prerequisites
1. **Operating System**: Windows 10 (Build 1809+) or Windows 11 x64.
2. **Visual Studio C++ Build Tools**:
   - Visual Studio 2022 Build Tools with "Desktop development with C++".
   - Windows 10 / 11 SDK (version 10.0.19041.0 or newer).
3. **Rust Toolchain**:
   - Install Rust via `rustup` configured for the MSVC ABI:
     ```powershell
     rustup default stable-x86_64-pc-windows-msvc
     rustup update
     ```
   - The workspace specifies `edition = "2024"` and `rust-version = "1.85.0"`.
4. **Docker & Docker Compose**: For running local PostgreSQL and MinIO instances.

### 1.2 Repository Structure
```
trajectory-recorder/
├── Cargo.toml                       # Workspace manifest
├── rust-toolchain.toml              # Rust toolchain pin
├── apps/
│   ├── supervisor/                  # trajectory-supervisor.exe (Session 0 Service)
│   ├── capture-agent/               # trajectory-agent.exe (Interactive Desktop Agent)
│   ├── uploader/                    # trajectory-uploader.exe (Chunk Uploader)
│   ├── browser-host/                # trajectory-browser-host.exe (Native Messaging Host)
│   ├── server/                      # trajectory-server (Axum Ingestion Server)
│   └── desktop-ui/                  # trajectory-tray.exe (Tauri 2 Tray / UI)
├── crates/                          # 19 Modular Domain Crates
│   ├── core-types/                  # Foundation types, schemas, timestamps
│   ├── config/                      # TOML configuration schemas & manager
│   ├── ipc/                         # Named Pipe MessagePack IPC & SDDL
│   ├── event-bus/                   # High-throughput bounded MPMC bus
│   ├── input-win/                   # Win32 low-level mouse & keyboard hooks
│   ├── window-win/                  # SetWinEventHook foreground & monitor tracker
│   ├── uia-win/                     # Dedicated COM STA UIAutomation engine
│   ├── capture-win/                 # WebP screenshot pipeline & video loop
│   ├── privacy/                     # 3-Tier privacy redaction engine
│   ├── correlator/                  # Action synthesis & burst grouping
│   ├── session/                     # Hourly partition manager, SQLite WAL, NDJSON
│   ├── spool/                       # 6-stage Spool state machine & disk watchdog
│   ├── archive/                     # TAR.Zstd streaming compression & chunking
│   ├── crypto/                      # DPAPI & XChaCha20-Poly1305 AEAD
│   └── upload-client/               # Resumable HTTP client with backoff
├── browser-extension/               # Chrome/Edge Manifest V3 Extension
├── server/                          # Server migrations & docker-compose.yml
└── tests/                           # Testing Infrastructure
    ├── harness-app/                 # trajectory-harness.exe (UI Test Fixture)
    ├── e2e-runner/                  # Opaque-box E2E test runner
    └── tier1-feature/               # Tier 1 Feature Coverage test suite
```

---

## 2. Local Infrastructure: PostgreSQL & MinIO

The workspace provides a `docker-compose.yml` to spin up local database and object storage dependencies:

```bash
# Start PostgreSQL 16 and MinIO in the background
docker compose -f server/docker-compose.yml up -d

# Verify container status
docker compose -f server/docker-compose.yml ps
```

### Environment Configuration (`server/.env`)
Copy the template configuration to `server/.env`:
```bash
cp server/.env.example server/.env
```

Default contents:
```ini
DATABASE_URL=postgres://postgres:postgres@localhost:5432/trajectory_db
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=trajectory-archives
S3_REGION=us-east-1
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
JWT_SECRET=dev_secret_key_change_in_production_32_chars_long
BIND_ADDR=0.0.0.0:8080
```

---

## 3. Building & Running Components

### 3.1 Cargo Workspace Commands

```bash
# Typecheck all crates and binaries
cargo check --workspace

# Run all unit and integration tests
cargo test --workspace

# Build all binaries in debug mode
cargo build --workspace

# Build optimized production release binaries
cargo build --workspace --release
```

### 3.2 Running the Development Stack Locally

#### 1. Ingestion Server
```bash
cargo run --bin trajectory-server
```
The server listens on `http://localhost:8080` and exposes health check endpoint `GET /api/v1/health`.

#### 2. Capture Agent (Interactive Session)
```bash
cargo run --bin trajectory-agent
```
Starts capturing mouse, keyboard, active window, UIA elements, and screenshots into `./spool/recording/`.

#### 3. Background Uploader
```bash
cargo run --bin trajectory-uploader
```
Monitors `./spool/pending_upload/`, compresses sessions to TAR.Zstd, encrypts chunks, and streams them to the ingestion server.

#### 4. Supervisor (Windows Service Mode or Console Mode)
```bash
# Run in console mode for development
cargo run --bin trajectory-supervisor

# Install as Windows Service (Administrator prompt required)
target\release\trajectory-supervisor.exe --install-service
```

---

## 4. Testing Framework & Test Fixture (`trajectory-harness.exe`)

The testing architecture follows an **opaque-box testing philosophy** defined in `TEST_INFRA.md`.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          E2E TEST ORCHESTRATION                             │
│                                                                             │
│  ┌──────────────────────┐  Injected Hardware Inputs ┌─────────────────────┐ │
│  │   e2e-runner         │──────────────────────────►│ trajectory-harness  │ │
│  │ (Verification Suite) │                           │ (UI Test Fixture)   │ │
│  └──────────┬───────────┘                           └──────────┬──────────┘ │
│             │                                                  │            │
│             │                                     Observed UI  │            │
│             │                                     Interactions │            │
│             ▼                                                  ▼            │
│  ┌──────────────────────┐  Hourly Spool Artifacts   ┌─────────────────────┐ │
│  │ Artifact Verifiers   │◄──────────────────────────│  trajectory-agent   │ │
│  │ ├─ NDJSON Validator  │ (events.raw.ndjson)       │  (Capture Engine)   │ │
│  │ ├─ SQLite Inspector  │ (events.normalized.ndjson)└─────────────────────┘ │
│  │ ├─ WebP Diff Engine  │ (session.db)                                      │
│  │ └─ AEAD Decryptor    │ (manifest.json)                                   │
│  └──────────────────────┘                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Running the Interactive Test Fixture
```bash
# Launch test fixture GUI
cargo run -p harness-app -- --mode interactive

# Launch test fixture in headless mode for CI
cargo run -p harness-app -- --mode headless
```

### 4.2 Standard UI Controls Exposed in `harness-app`
- `btn_submit` (`UIA_ButtonControlTypeId`): Standard submit button.
- `txt_username` (`UIA_EditControlTypeId`): Plaintext edit box.
- `txt_password` (`UIA_EditControlTypeId`): Secure edit with `IsPassword = true`.
- `txt_credit_card` (`UIA_EditControlTypeId`): Validates Luhn credit card redaction.
- `pnl_scrollable` (`UIA_PaneControlTypeId`): Multi-row virtualized scrolling panel.
- `drag_source` / `drop_target`: Drag-and-drop interaction canvas.

### 4.3 Running Specific Test Suites
```bash
# Run all Tier 1 Feature Coverage tests
cargo test -p tier1-feature -- --nocapture

# Run Privacy Engine Redaction test
cargo test -p tier1-feature --test test_f18_privacy_redaction -- --nocapture

# Run Dual Timestamps test
cargo test -p tier1-feature --test test_f02_dual_timestamps -- --nocapture

# Run E2E verification engines
cargo test -p e2e-runner -- --nocapture
```

---

## 5. Database Migrations

Server relational database schema migrations reside in `server/migrations/`:
- `0001_initial_schema.sql`: Table definitions for `machines`, `sessions`, `session_chunks`, and `machine_heartbeats`.
- `0002_indexes.sql`: Performance indices for session querying and upload tracking.

Migrations are applied automatically when the Axum server connects to PostgreSQL via `sqlx::migrate!()`.

---

## 6. Production Quality Standards & Coding Rules

To maintain high stability and audit compliance, all code contributed to this repository must strictly adhere to the following rules:

### Rule 1: No Unhandled `unwrap()` in Production Paths
- ❌ **Prohibited**: `let val = result.unwrap();` or `option.unwrap()` in capture, persistence, or network paths.
- ✅ **Required**: Use `?`, `match`, `if let`, or `map_err` with structured error logging.

### Rule 2: No Hardcoded Secrets
- ❌ **Prohibited**: Embedding JWT signing keys, database passwords, or S3 credentials in source code.
- ✅ **Required**: Read secrets from environment variables (`DATABASE_URL`, `JWT_SECRET`, `S3_SECRET_KEY`) or Windows DPAPI.

### Rule 3: Mandatory Safety Invariant Comments on `unsafe` Blocks
Every `unsafe` block interacting with the Win32 API must include a `// SAFETY:` comment explaining the memory invariant:
```rust
// SAFETY:
// 1. `out_blob` is zero-initialized and allocated on the stack.
// 2. `CryptProtectData` guarantees valid memory writes to `out_blob.pbData`.
// 3. The allocated buffer is freed via `LocalFree` immediately after copying.
let success = unsafe {
    CryptProtectData(
        &mut in_blob,
        PCWSTR(ptr::null()),
        None,
        None,
        None,
        flags,
        &mut out_blob,
    )
};
```

### Rule 4: Win32 API Return Code & Error Handling
- Always check Win32 `BOOL` results, `HRESULT`, and `INVALID_HANDLE_VALUE`.
- Capture error codes immediately via `unsafe { GetLastError() }` on failure.

### Rule 5: Unit Test Coverage for Every Crate
- Every crate in `crates/` must maintain unit tests verifying its primary public API and edge cases (boundary values, empty inputs, corrupted data).

---

## 7. Troubleshooting & Diagnostics

### 7.1 Inspecting Active Spool Directories
```powershell
# List sessions across all spool stages
Get-ChildItem -Path .\spool -Recurse -Depth 2
```

### 7.2 Validating SQLite Session Database
```powershell
# Check database integrity and examine canonical action counts
sqlite3 .\spool\recording\<session_id>\session.db "PRAGMA integrity_check;"
sqlite3 .\spool\recording\<session_id>\session.db "SELECT count(*) FROM canonical_actions;"
```

### 7.3 Verifying NDJSON Line Integrity
```powershell
# Count lines and inspect first record
Get-Content .\spool\recording\<session_id>\events.raw.ndjson | Measure-Object -Line
Get-Content .\spool\recording\<session_id>\events.normalized.ndjson -Head 1 | ConvertFrom-Json
```
