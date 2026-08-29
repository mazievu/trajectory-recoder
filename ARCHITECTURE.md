# Trajectory Recorder: Architecture & System Design

**Target Platform**: Windows 10/11 x64  
**Implementation Language**: Rust (Edition 2024, `1.85.0+`)  
**Document Classification**: Production Technical Architecture  

---

## Table of Contents
1. [Architectural Overview & Core Principles](#1-architectural-overview--core-principles)
2. [Process Isolation & Topology](#2-process-isolation--topology)
3. [Inter-Process Communication (IPC) Protocol](#3-inter-process-communication-ipc-protocol)
4. [High-Throughput Concurrency & Threading Model](#4-high-throughput-concurrency--threading-model)
5. [End-to-End Pipeline & Event Flow](#5-end-to-end-pipeline--event-flow)
6. [Session Lifecycle & 6-Stage Spool State Machine](#6-session-lifecycle--6-stage-spool-state-machine)
7. [Security & Cryptographic Architecture](#7-security--cryptographic-architecture)
8. [Failure Modes & Recovery Semantics](#8-failure-modes--recovery-semantics)

---

## 1. Architectural Overview & Core Principles

The **Trajectory Recorder** is a distributed, continuous multimodal desktop interaction capture and reconstruction platform. It captures granular low-level hardware inputs (mouse, keyboard), window lifecycle changes, semantic UI element trees (via UI Automation), multi-monitor visual evidence (WebP screenshots and video), clipboard events, and browser DOM interactions without perturbing the host user experience.

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                   HOST WORKSTATION                                      │
│                                                                                         │
│  [ Session 0 (SYSTEM) ]                                                                 │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │ trajectory-supervisor.exe                                                         │  │
│  │ ├─ DPAPI Token Storage   ├─ Startup Crash Recovery   ├─ 4-Tier Disk Watchdog      │  │
│  └─────────────────────────────────────┬─────────────────────────────────────────────┘  │
│                                        │ Named Pipe IPC (rmp-serde)                     │
│  [ Interactive Session (User) ]        ▼                                                │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │ trajectory-agent.exe                                                              │  │
│  │ ├─ Win32 Hooks (WH_MOUSE_LL, WH_KEYBOARD_LL) ──┐                                  │  │
│  │ ├─ Window Tracker (SetWinEventHook)           ──┼─► Bounded Event Bus (MPMC)      │  │
│  │ ├─ UI Automation Worker (Dedicated STA COM)   ──┤         │                       │  │
│  │ ├─ WebP Screen Capture & Video Loop           ──┤         ▼                       │  │
│  │ ├─ Clipboard & File Watchers                  ──┘   Privacy Engine (In-Memory)    │  │
│  │ ├─ Action Correlator (Burst & State Builder)              │                       │  │
│  │ └─ Session Router (Hourly Rotation) ──────────────────────┴─► Local Spool         │  │
│  └─────────────────────────────────────▲─────────────────────────────┬───────────────┘  │
│                                        │ Named Pipe                  │                  │
│  ┌─────────────────────────────────────┴─────────────┐               │                  │
│  │ trajectory-browser-host.exe ◄── Chrome/Edge (MV3) │               │                  │
│  └───────────────────────────────────────────────────┘               │                  │
│                                                                      ▼                  │
│  [ Background Worker ]                                     ┌──────────────────┐         │
│  ┌───────────────────────────────────────────────────────┐ │ Local Spool FSM  │         │
│  │ trajectory-uploader.exe                               │ │ (recording/ ->   │         │
│  │ ├─ TAR + Streaming Zstandard Compression              │ │  uploaded/)      │         │
│  │ ├─ XChaCha20-Poly1305 AEAD Chunk Encryption (64 MiB)  │ └────────┬─────────┘         │
│  │ └─ Resumable HTTP Client (Jittered Backoff) ◄─────────┼──────────┘                   │
│  └───────────────────────────┬───────────────────────────┘                              │
└──────────────────────────────┼──────────────────────────────────────────────────────────┘
                               │ HTTPS REST (TLS 1.3)
                               ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                               CLOUD / INGESTION CLUSTER                                 │
│                                                                                         │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │ trajectory-server (Axum + Tokio)                                                  │  │
│  │ ├─ JWT Machine Authentication                                                     │  │
│  │ ├─ Chunk Streaming & On-The-Fly SHA-256 Validation                                │  │
│  │ ├─ PostgreSQL 16+ (Metadata, Sessions, Chunks, Metrics)                           │  │
│  │ └─ S3 / MinIO Object Storage (Partitioned Encrypted Chunks)                       │  │
│  └───────────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Key Architectural Tenets
1. **Zero Impact on User Experience**: Hook message pumps and event dispatchers execute asynchronously in less than $50\ \mu\text{s}$, strictly avoiding any UI thread blocking or mouse cursor stutter.
2. **Crash-Resilience by Design**: All streaming telemetry is appended to local NDJSON and SQLite WAL files with synchronized flush intervals $< 2\text{ seconds}$. Hard power loss loses at most 2 seconds of uncommitted tail data.
3. **In-Memory Privacy Guarantee**: Sensitive data (passwords, credit card numbers, SSNs, API tokens, and high-entropy strings) is redacted in-memory prior to any persistence or IPC transmission.
4. **Resilient Network Decoupling**: Workstations can operate fully offline for days. Local 4-tier disk pressure watermarks dynamically manage retention, and the uploader automatically drains backlogs when network connectivity is restored.

---

## 2. Process Isolation & Topology

Windows architecture enforces strict separation between background services (**Session 0**) and interactive desktop sessions (**Session 1+**). Trajectory Recorder respects these OS boundaries:

```
+-------------------+--------------------+-----------------------+------------------------+
| Process Name      | Windows Session    | Security Context      | Primary Responsibility |
+-------------------+--------------------+-----------------------+------------------------+
| supervisor        | Session 0          | NT AUTHORITY\SYSTEM   | Daemon lifecycle,      |
|                   | (Non-interactive)  |                       | DPAPI machine secrets, |
|                   |                    |                       | disk watchdog, crash   |
|                   |                    |                       | recovery scanner.      |
+-------------------+--------------------+-----------------------+------------------------+
| capture-agent     | Session 1..N       | Interactive User      | Desktop Win32 hooks,   |
|                   | (Active GUI)       | (Standard or Admin)   | UIA tree walker,       |
|                   |                    |                       | screenshots, NDJSON &  |
|                   |                    |                       | SQLite persistence.    |
+-------------------+--------------------+-----------------------+------------------------+
| browser-host      | Session 1..N       | Interactive User      | Native Messaging Host  |
|                   | (Active GUI)       |                       | bridge between Chrome/ |
|                   |                    |                       | Edge MV3 and agent.    |
+-------------------+--------------------+-----------------------+------------------------+
| uploader          | Session 1..N or 0  | Interactive / SYSTEM  | Spool compressor,      |
|                   | (Background)       |                       | XChaCha20 encryptor,   |
|                   |                    |                       | resumable HTTP client. |
+-------------------+--------------------+-----------------------+------------------------+
| desktop-ui (tray) | Session 1..N       | Interactive User      | Tauri 2 UI, status     |
|                   | (Active GUI)       |                       | tray, timeline viewer. |
+-------------------+--------------------+-----------------------+------------------------+
```

### Why Session 0 vs. Interactive Session?
- **Session 0 Isolation**: Since Windows Vista, services running in Session 0 cannot interact directly with the user's desktop, receive window messages, or install global input hooks (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`).
- **The Supervisor Architecture**: `trajectory-supervisor` runs as a standard Windows Service in Session 0. It monitors system health, starts user-session capture agents upon logon, and guards machine-level cryptographic keys.
- **The Agent Architecture**: `trajectory-agent` runs in the active user desktop session with full access to Win32 message queues, DirectX/GDI displays, and UI Automation provider trees.

---

## 3. Inter-Process Communication (IPC) Protocol

Processes communicate across session boundaries using **Windows Named Pipes** serialized with **MessagePack (`rmp-serde`)**.

### Pipe Endpoints
- `\\.\pipe\trajectory-agent-ipc`: Primary control channel between `supervisor` and `capture-agent`.
- `\\.\pipe\trajectory-browser-host`: Communication bridge from `trajectory-browser-host` to `capture-agent`.
- `\\.\pipe\trajectory-tray-ipc`: Status query channel between `trajectory-tray` and `capture-agent`.

### Framing & Serialization
Every IPC frame consists of a 4-byte big-endian payload length header followed by the binary MessagePack payload:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Payload Length (32-bit uint)                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|             MessagePack Payload (rmp-serde format)            |
|                     (Up to 64 MiB maximum)                    |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Security Descriptor Definition Language (SDDL)
To prevent unauthorized local processes from connecting or injecting commands into the pipes, all Named Pipes are created with explicit security attributes:

```sddl
D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;IU)
```
- `SY` (SYSTEM): Generic All (`GA`) access.
- `BA` (Built-in Administrators): Generic All (`GA`) access.
- `IU` (Interactive Users): Generic All (`GA`) access.
- **Network / Remote Access**: Completely forbidden (default local-only pipe semantics).

---

## 4. High-Throughput Concurrency & Threading Model

To handle high-frequency events (1000 Hz mouse inputs, 60 FPS window redraws) without dropping frames or introducing latency, `trajectory-agent` uses a dedicated hybrid concurrency model:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          CAPTURE AGENT PROCESS                              │
│                                                                             │
│  ┌─────────────────────────┐  crossbeam-channel    ┌──────────────────────┐ │
│  │ Win32 Hook Thread (OS)  │──────────────────────►│ Async Tokio Runtime  │ │
│  │ (WH_MOUSE_LL pump)      │ (Non-blocking enqueue)│                      │ │
│  │ (WH_KEYBOARD_LL pump)   │                       │ ├─ Event Bus Router  │ │
│  └─────────────────────────┘                       │ ├─ Action Correlator │ │
│                                                    │ ├─ NDJSON Sink       │ │
│  ┌─────────────────────────┐  mpsc channel         │ ├─ SQLite WAL Worker │ │
│  │ Dedicated STA COM       │──────────────────────►│ ├─ IPC Server/Client │ │
│  │ UIAutomation Thread     │ (Async request/reply) │ └─ Disk Watchdog     │ │
│  │ (100ms Timeout Guard)   │                       │                      │ │
│  └─────────────────────────┘                       └──────────────────────┘ │
│                                                                             │
│  ┌─────────────────────────┐                       ┌──────────────────────┐ │
│  │ Screenshot / WGC Thread │──────────────────────►│ WebP Encoder Thread  │ │
│  │ (DirectX / GDI Capture) │   BGRA Raw Buffers    │ (libwebp / image)    │ │
│  └─────────────────────────┘                       └──────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1. Win32 Hook Message Pump (Native OS Thread)
- Runs a standard Win32 `GetMessage` / `DispatchMessage` pump.
- The hook callbacks (`LowLevelMouseProc`, `LowLevelKeyboardProc`) execute minimal logic: they read the timestamp, copy coordinate/key data, push to a lock-free `crossbeam_channel::Sender`, and immediately return `CallNextHookEx`.
- Processing budget: $< 50\ \mu\text{s}$ per hook invocation.

### 2. Dedicated Single-Threaded Apartment (STA) COM UIAutomation Worker
- Windows UI Automation (`IUIAutomation`) requires an initialized COM apartment. Calling UIA from multi-threaded Tokio worker pools causes deadlocks and RPC marshalling failures.
- A dedicated background thread initializes `CoInitializeEx(COINIT_APARTMENTTHREADED)`.
- Communication with Tokio runtime occurs via asynchronous channels. Calls to `ElementFromPoint` and parent hierarchy walks are guarded by a strict **100 ms timeout**. If an application freezes or deadlocks, the UIA worker aborts the call and returns fallback metadata without stalling the pipeline.

### 3. Asynchronous Tokio Runtime
- Coordinates event bus buffering, typing burst aggregation (500 ms debounce), scroll gesture aggregation (300 ms debounce), drag-and-drop state machines, NDJSON disk streaming, and IPC heartbeats.

---

## 5. End-to-End Pipeline & Event Flow

From the instant a user interacts with the workstation to the point an encrypted archive is accepted by the server, the data undergoes 7 distinct pipeline transformations:

```
[1. Hook / Hardware Interaction]
  • Mouse Click at (1420, 850) on Monitor 1
  • Raw timestamp: Utc::now() + QPC monotonic nanoseconds
        │
        ▼
[2. Enrichment & Semantic Resolution]
  • Window Tracker identifies active window: Excel.exe (HWND: 0x004A12F0)
  • STA UIA Worker resolves target element: Button "Save" (AutomationId: "btn_save")
  • Ancestor chain resolved: RibbonBar -> MainWindow -> Desktop
        │
        ▼
[3. In-Memory Privacy Redaction]
  • PrivacyEngine checks: IsPassword = false
  • Target text, clipboard, and input parameters scanned against Regex & Shannon entropy
  • Output: Cleaned CanonicalAction struct
        │
        ▼
[4. Action Correlation & Evidence Attachment]
  • Action Correlator synthesizes: CanonicalAction { action_type: CLICK, confidence: 1.0 }
  • Screen Capture captures `before.webp` and stabilized `after.webp`
  • Evidence attached: ScreenshotRef, WindowContext, TargetMetadata
        │
        ▼
[5. Local Partition Persistence]
  • Raw event appended to `spool/recording/{session_id}/events.raw.ndjson`
  • Canonical action appended to `spool/recording/{session_id}/events.normalized.ndjson`
  • Action record indexed in SQLite WAL `spool/recording/{session_id}/session.db`
        │ (Hourly Clock Boundary Trigger)
        ▼
[6. Packaging, Compression & Encryption]
  • Spool transitions: `recording/` -> `finalizing/` -> `pending_upload/`
  • Tar + Zstandard streaming compression creates `{session_id}.tar.zst`
  • Chunker slices archive into 64 MiB chunks: `chunk_0000.bin`, `chunk_0001.bin`
  • Each chunk encrypted with XChaCha20-Poly1305 AEAD using DPAPI-derived key
        │
        ▼
[7. Cloud Ingestion & Verification]
  • Uploader streams chunks via PUT `/api/v1/sessions/{id}/chunks/{idx}` with `X-Chunk-SHA256`
  • Server streams chunks directly into S3 bucket: `trajectory/{machine}/{YYYY}/{MM}/{DD}/{HH}/{id}/`
  • Server verifies cumulative archive SHA-256 and issues `SESSION_ACCEPTED`
```

---

## 6. Session Lifecycle & 6-Stage Spool State Machine

To guarantee zero data loss and idempotent upload resumes, session directories transition through a deterministic 6-stage finite state machine (FSM):

```
       ┌──────────────┐
       │  RECORDING   │ ◄── Active session (hourly partition)
       └──────┬───────┘
              │ Clock-hour boundary reached (e.g. 09:00:00)
              ▼
       ┌──────────────┐
       │  FINALIZING  │ ◄── Flushes NDJSON, checkpoints SQLite WAL, closes files
       └──────┬───────┘
              │ TAR.Zstd packaging & XChaCha20 encryption complete
              ▼
       ┌────────────────┐
       │ PENDING_UPLOAD │ ◄── Session chunks staged for upload
       └──────┬─────────┘
              │ Uploader initiates upload session with server
              ▼
       ┌──────────────┐
       │  UPLOADING   │ ◄── Resumable chunk streaming in progress
       └──────┬───────┘
              │
      ┌───────┴─────────────────┐
      │ All chunks verified     │ Chunk mismatch or unrecoverable error
      ▼                         ▼
┌────────────┐            ┌────────────┐
│  UPLOADED  │            │   FAILED   │
└─────┬──────┘            └─────┬──────┘
      │                         │
      │ Purged when disk        │ Evaluated for retry
      │ watermark > 85%         │ by Supervisor
      ▼                         ▼
  [ Deleted ]               [ Retry ]
```

### Directory States & Paths
- `spool/recording/{session_id}/`: Actively written by `trajectory-agent`. Contains `manifest.json` (status: `RECORDING`), `events.raw.ndjson`, `events.normalized.ndjson`, `session.db`, and `screenshots/`.
- `spool/finalizing/{session_id}/`: Transitioned atomically via `std::fs::rename`. No more events are accepted. Compression worker packages directory into TAR.Zstd archive.
- `spool/pending_upload/{session_id}/`: Contains encrypted chunk files (`chunk_0000.bin`...) and `manifest.json`.
- `spool/uploading/{session_id}/`: Currently actively uploading to cloud ingestion API.
- `spool/uploaded/{session_id}/`: Successfully acknowledged by server (`SESSION_ACCEPTED`). Stored locally until purged by disk retention policy.
- `spool/failed/{session_id}/`: Sessions that encountered non-retryable corruption or rejected authentication.

---

## 7. Security & Cryptographic Architecture

The security model employs defense-in-depth across storage, computation, and transmission:

```
+--------------------------+-----------------------------+------------------------------------+
| Layer                    | Cryptographic Primitive     | Purpose / Description              |
+--------------------------+-----------------------------+------------------------------------+
| Machine Secret Storage   | Windows DPAPI               | Master encryption key protected    |
|                          | (CryptProtectData)          | with CRYPTPROTECT_LOCAL_MACHINE    |
+--------------------------+-----------------------------+------------------------------------+
| Local Staged Chunks      | XChaCha20-Poly1305 AEAD     | 256-bit authenticated encryption   |
|                          | (24-byte random nonces)     | with AAD binding session & chunk # |
+--------------------------+-----------------------------+------------------------------------+
| Integrity Verification   | SHA-256 Digest              | Per-chunk and cumulative archive   |
|                          |                             | cryptographic integrity check      |
+--------------------------+-----------------------------+------------------------------------+
| Transport Layer          | TLS 1.3                     | HTTPS REST upload transport        |
+--------------------------+-----------------------------+------------------------------------+
| API Authentication       | JSON Web Tokens (JWT)       | Signed device authentication tokens|
|                          | (HMAC-SHA256)               | issued on machine registration     |
+--------------------------+-----------------------------+------------------------------------+
| In-Memory Privacy        | Shannon Entropy + Regex     | Real-time redaction of credentials |
|                          | + Luhn Algorithm            | before writing to disk or network  |
+--------------------------+-----------------------------+------------------------------------+
```

---

## 8. Failure Modes & Recovery Semantics

| Failure Scenario | System Reaction & Recovery Protocol |
|---|---|
| **Abrupt Process Termination (`taskkill /F`)** | Upon restart, `trajectory-supervisor` scans `spool/recording/`. Corrupted trailing NDJSON lines ($< 2\text{s}$) are cleanly truncated, SQLite WAL is recovered via `sqlite3_recover`, and the session is marked `RECOVERED` and moved to `pending_upload/`. |
| **Workstation Power Loss / Hard Reboot** | Monotonic session IDs and crash-safe global ID block allocations (`global_event_id.dat`) ensure no event ID reuse occurs upon reboot. |
| **Network Disconnection / Outage** | Sessions queue in `pending_upload/`. When connectivity resumes, `trajectory-uploader` queries `/upload-status` and resumes from the exact missing chunk index using exponential backoff with jitter. |
| **Disk Space Exhaustion (> 85% / > 92%)** | 4-Tier Disk Watchdog triggers: Level 1 (70%) enables aggressive compression; Level 2 (85%) purges oldest `uploaded/` sessions; Level 3 (92%) throttles screenshot capture and alerts supervisor. |
| **UI Automation Slow / Unresponsive App** | 100 ms timeout guard on COM STA thread aborts stalled UIA queries, falling back to window title and coordinates without dropping input stream. |
