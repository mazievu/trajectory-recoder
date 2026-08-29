# TRAJECTORY RECORDER: TEST INFRASTRUCTURE & E2E VERIFICATION SPECIFICATION

**Document Version**: 1.0.0  
**Target Environment**: Windows 10/11 x64, Rust Edition 2024 (`x86_64-pc-windows-msvc`), Axum Ingestion Server, PostgreSQL 16+, MinIO / S3  
**Standards Compliance**: ISO/IEC/IEEE 29119 Software Testing, Sections 0–75 of Master Spec, 40 Acceptance Criteria (AC 1–AC 40)

---

# 1. EXECUTIVE SUMMARY & OBJECTIVES

The **Trajectory Recorder Test Infrastructure** provides a completely deterministic, reproducible, multi-tiered verification framework designed to guarantee 100% functional, performance, security, and resilience compliance across the entire Trajectory Recorder ecosystem.

The test infrastructure encompasses:
1. **Opaque-Box Testing Philosophy**: All tests operate against black-box observable interfaces (Win32 inputs, UI Automation tree, DOM bridge, local spool files, SQLite databases, encrypted TAR.Zstd archives, and REST API endpoints) without coupling to internal crate implementation details.
2. **Standardized Native Test Fixture (`harness-app`)**: A standalone Win32 / UIA application (`trajectory-harness.exe`) providing deterministic, zero-dependency UI controls (buttons, inputs, passwords, dropdowns, scroll containers, drag zones, file dialogs, and async loading spinners).
3. **Automated E2E Orchestration & Verification Runner (`e2e-runner`)**: A high-performance test engine that launches system processes, injects synthetic hardware inputs, manages temporary spool directories, inspects NDJSON/SQLite databases, validates WebP screenshots, tests mock REST endpoints, and verifies fault recovery.
4. **Multi-Tier Testing Hierarchy (Tiers 1–5)**:
   - **Tier 1 (Feature Coverage)**: Comprehensive equivalence-class testing for all 50 features across the system.
   - **Tier 2 (Boundary Value Analysis)**: Exact boundary testing for clock-hour boundaries, chunk sizes (64–256 MiB), 4-tier disk pressure thresholds (70%, 85%, 92%), queue capacities, and 100ms UIA timeouts.
   - **Tier 3 (Pairwise Combinatorial Matrix)**: Multi-variable interaction tests across application types, input modalities, privacy rules, disk tiers, and network connectivity states.
   - **Tier 4 (Real-World Workload & Soak Scenarios)**: 30-minute cross-application workflow (Chrome → Excel → Explorer → Photoshop → Chrome across hour boundaries) and 8-hour soak stability testing.
   - **Tier 5 (Fault Injection & Adversarial Resilience)**: Sudden `SIGKILL` / `taskkill /F`, corrupt SQLite WAL recovery, encrypted chunk tampering, and offline network backlog draining.

---

# 2. TEST SUITE ARCHITECTURE & DIRECTORY LAYOUT

The test framework is organized within the `tests/` directory as part of the Cargo workspace:

```
trajectory-recorder/
├── tests/
│   ├── harness-app/                     # Standardized Windows UI Test Fixture
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                  # Win32 window message loop & UIA provider
│   │       ├── controls/                # Standard UI controls (buttons, inputs, dialogs)
│   │       │   ├── mod.rs
│   │       │   ├── button_view.rs       # Normal, toggle, radio, and submit buttons
│   │       │   ├── input_view.rs        # Text, multiline, numeric, and password inputs
│   │       │   ├── scroll_view.rs       # Nested virtualized scrolling panels
│   │       │   ├── drag_view.rs         # Drag source & drop target surface
│   │       │   ├── dialog_view.rs       # Modal popups, Open/Save common dialogs
│   │       │   └── progress_view.rs     # Deterministic async spinners and progress bars
│   │       ├── driver.rs                # Programmatic CLI & automation driver
│   │       └── uia_provider.rs          # Accessible UIA AutomationId & Role provider
│   │
│   ├── e2e-runner/                      # E2E Test Engine & Verification Library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                   # Core E2E runner exports
│   │       ├── harness_client.rs        # Controller for launching & driving harness-app
│   │       ├── agent_controller.rs      # Supervisor & Agent lifecycle manager
│   │       ├── verifiers/               # Artifact inspection engines
│   │       │   ├── mod.rs
│   │       │   ├── ndjson_verifier.rs   # Raw & Normalized NDJSON syntax & sequence validator
│   │       │   ├── sqlite_verifier.rs   # SQLite WAL schema, indexes & record checker
│   │       │   ├── screenshot_verifier.rs # WebP image decoding & perceptual diff validator
│   │       │   ├── archive_verifier.rs  # TAR.Zstd unpacker & manifest checksum verifier
│   │       │   ├── crypto_verifier.rs   # DPAPI & XChaCha20-Poly1305 decryption tester
│   │       │   └── upload_verifier.rs   # REST API chunk validator & mock Ingestion Server
│   │       ├── mock_server.rs           # Embedded Axum Ingestion Server for upload testing
│   │       └── scenario.rs              # Scenario builder & execution runner
│   │
│   ├── tier1-feature/                   # Tier 1 Feature Coverage Test Suite
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── tests/
│   │       ├── test_f01_workspace_foundation.rs
│   │       ├── test_f02_dual_timestamps.rs
│   │       ├── test_f03_core_schemas.rs
│   │       ├── test_f04_config_management.rs
│   │       ├── test_f05_named_pipe_ipc.rs
│   │       ├── test_f06_dpapi_security.rs
│   │       ├── test_f07_mouse_capture.rs
│   │       ├── test_f08_keyboard_capture.rs
│   │       ├── test_f09_window_tracking.rs
│   │       ├── test_f10_event_bus_priority.rs
│   │       ├── test_f11_ndjson_persistence.rs
│   │       ├── test_f12_clipboard_monitoring.rs
│   │       ├── test_f13_file_telemetry.rs
│   │       ├── test_f14_uia_semantic_capture.rs
│   │       ├── test_f15_typing_burst_grouping.rs
│   │       ├── test_f16_scroll_burst_grouping.rs
│   │       ├── test_f17_drag_drop_synthesis.rs
│   │       ├── test_f18_privacy_redaction.rs
│   │       ├── test_f19_screenshot_evidence.rs
│   │       ├── test_f20_video_fragment_sync.rs
│   │       ├── test_f21_session_lifecycle.rs
│   │       ├── test_f22_spool_state_machine.rs
│   │       ├── test_f23_archive_compression.rs
│   │       ├── test_f24_crypto_encryption.rs
│   │       └── test_f25_upload_pipeline.rs
│   │
│   ├── tier2-boundary/                  # Tier 2 Boundary Value Analysis (BVA)
│   ├── tier3-pairwise/                  # Tier 3 Pairwise Combinatorial Matrix
│   └── tier4-workload/                  # Tier 4 30-Minute & Soak Stress Scenarios
```

---

# 3. STANDARDIZED TEST FIXTURE: `trajectory-harness.exe`

The `harness-app` provides a clean, predictable Windows application designed to exercise every interaction modality captured by `trajectory-agent`.

### 3.1 Control Inventory & Accessibility Specifications

| Control ID / AutomationId | UI Type | Exposed UIA Role / ControlType | Injected Behavioral Response |
|---|---|---|---|
| `btn_submit` | Push Button | `UIA_ButtonControlTypeId` | Emits click event, updates status label to `"Submitted"`. |
| `btn_toggle` | Toggle Button | `UIA_ButtonControlTypeId` | Cycles state `Checked` / `Unchecked`. |
| `txt_username` | Single-Line Edit | `UIA_EditControlTypeId` | Stores typed string; `IsPassword = false`. |
| `txt_password` | Password Edit | `UIA_EditControlTypeId` | `IsPassword = true`, verifies Privacy Engine masks value to `"[REDACTED]"`. |
| `txt_credit_card` | Formatted Input | `UIA_EditControlTypeId` | Formats credit card digits; verifies Luhn algorithm masking. |
| `txt_notes` | Multi-Line Edit | `UIA_EditControlTypeId` | Supports Enter, Tab, and multiline text bursts. |
| `cmb_category` | Combo Box | `UIA_ComboBoxControlTypeId` | Opens dropdown list with selectable items on click. |
| `pnl_scrollable` | Scroll Viewport | `UIA_PaneControlTypeId` | Contains 500 rows with vertical & horizontal scrollbars. |
| `drag_source` | Canvas Element | `UIA_CustomControlTypeId` | Initiates OLE / Win32 drag operation on mouse down + drag. |
| `drop_target` | Canvas Element | `UIA_CustomControlTypeId` | Accepts dropped item, emits visual drop confirmation. |
| `dlg_open_file` | Button / Dialog | `UIA_ButtonControlTypeId` | Triggers Win32 Common File Dialog (`GetOpenFileNameW`). |
| `spinner_async` | Loading Spinner | `UIA_ProgressBarControlTypeId` | Simulates 1.8s system wait delay with active animation. |

### 3.2 Programmatic Automation Interface

`trajectory-harness.exe` can be executed interactively or operated via command-line arguments and named pipes:
- `--mode headless`: Runs without rendering window (for CI environments with mock graphics).
- `--mode interactive`: Displays full GUI with visual feedback.
- `--script <scenario.json>`: Automatically executes a sequence of scripted UI actions (clicks, typing, dialogs, scrolls) with microsecond timing.
- `--port <pipe_name>`: Listens on Named Pipe for real-time remote test driver commands.

---

# 4. VERIFICATION ENGINES & ASSERTION LIBRARIES

The `e2e-runner` contains dedicated verification modules ensuring every output artifact conforms strictly to the specification:

### 4.1 NDJSON Stream Verifier (`ndjson_verifier.rs`)
- Validates that every line is a valid JSON object matching the `RawEvent` or `CanonicalAction` schema.
- Asserts strict monotonic ordering of `DualTimestamp.monotonic_ns` and `global_event_id`.
- Asserts that all sensitive field inputs contain `"[REDACTED]"` or `"[UNOBSERVED_TEXT]"` and zero raw passwords/credit cards.
- Asserts that `events.raw.ndjson` and `events.normalized.ndjson` cross-reference matching `global_event_id` sequences.

### 4.2 SQLite WAL Verifier (`sqlite_verifier.rs`)
- Opens `index.sqlite` and executes schema integrity checks (`PRAGMA integrity_check`).
- Verifies existence and index constraints across all 7 core tables:
  `session_meta`, `raw_events`, `canonical_actions`, `screenshots`, `video_segments`, `annotations`, `id_allocator`.
- Asserts that total rows in `canonical_actions` match the count in `events.normalized.ndjson`.

### 4.3 Screenshot & Visual Evidence Verifier (`screenshot_verifier.rs`)
- Reads WebP image files from `screenshots/`.
- Decodes WebP bitstreams, verifying valid headers, dimensions, and color depths.
- Computes perceptual image diffs between `before.webp` and `after.webp` to confirm state changes.
- Verifies that clicked coordinate bounding boxes fall within the corresponding monitor geometry.

### 4.4 Archive & Compression Verifier (`archive_verifier.rs`)
- Inspects finalized `.tar.zst` packages.
- Validates streaming multi-threaded Zstandard decompression.
- Unpacks TAR stream and verifies manifest checksums against unpacked files:
  `manifest.json`, `events.raw.ndjson`, `events.normalized.ndjson`, `index.sqlite`, and `screenshots/*`.

### 4.5 Cryptographic Verifier (`crypto_verifier.rs`)
- Tests DPAPI key protection with `CryptProtectData` / `CryptUnprotectData`.
- Verifies that `.trajectory.enc` files are authenticated XChaCha20-Poly1305 ciphertexts.
- Asserts that tampered ciphertext bytes immediately fail decryption with authentication tag mismatch errors.

### 4.6 Mock Axum Ingestion Server (`mock_server.rs` & `upload_verifier.rs`)
- Runs an in-memory Axum HTTP server exposing the complete Ingestion REST API:
  - `POST /api/v1/machines/register`
  - `POST /api/v1/machines/heartbeat`
  - `POST /api/v1/sessions`
  - `PUT /api/v1/sessions/{id}/chunks/{index}` (with `X-Chunk-SHA256` validation)
  - `GET /api/v1/sessions/{id}/upload-status`
  - `POST /api/v1/sessions/{id}/complete`
- Validates chunk slicing, SHA-256 header hashing, retry backoff with jitter, and idempotent resume.

---

# 5. THE 19-ATTRIBUTE RECONSTRUCTION AUDIT CHECKLIST

To satisfy **AC 40** (30-Minute E2E Cross-Application Reconstruction), the test runner executes an automated audit verifying all 19 workflow attributes from trajectory artifacts:

```
+-----------------------------------------------------------------------------------+
|               19-ATTRIBUTE WORKFLOW RECONSTRUCTION AUDIT MATRIX                   |
+----+----------------------------+-------------------------------------------------+
| #  | Workflow Attribute         | Required Verifiable Trajectory Evidence         |
+----+----------------------------+-------------------------------------------------+
| 1  | Application Launches       | APP_OPEN with PID, exe path, timestamp          |
| 2  | Application Switches       | WINDOW_SWITCH with HWND, title, PID changes     |
| 3  | Active Window States       | Placement geometry (x,y,w,h) & monitor ID       |
| 4  | Clicked UI Targets         | CLICK with UIA AutomationId & DOM selector      |
| 5  | Typed Text Inputs          | TYPE_TEXT burst with privacy redaction applied  |
| 6  | Keyboard Shortcuts         | SHORTCUT (e.g. Ctrl+C, Ctrl+V, Ctrl+S)          |
| 7  | Clipboard Copy Sources     | COPY with source PID, exe, format, SHA-256      |
| 8  | Clipboard Paste Targets    | PASTE with dest PID, exe, matching data hash    |
| 9  | Opened Files               | FILE_OPEN with path, extension, file size       |
| 10 | Selected Files             | DIALOG_CONFIRM with selected file dialog path   |
| 11 | Uploaded Files             | FILE_UPLOAD correlating DOM input to disk path  |
| 12 | Downloaded Files           | FILE_DOWNLOAD with destination directory & size |
| 13 | Drag & Drop Source/Dest    | DRAG_DROP with source element and drop zone     |
| 14 | Scroll Containers          | SCROLL with container AutomationId & delta      |
| 15 | Appeared Dialogs           | DIALOG_OPEN with dialog title & control tree    |
| 16 | Dialog Confirmations       | DIALOG_CONFIRM / DIALOG_CANCEL choices          |
| 17 | System Wait Durations      | WAIT duration correlated to spinner UIA state   |
| 18 | Result & State Changes     | STATE_CHANGE capturing toast/DOM mutation text  |
| 19 | Final Workflow Output      | Terminal published state in session manifest    |
+----+----------------------------+-------------------------------------------------+
```

---

# 6. PASS / FAIL SEMANTICS & OPERATIONAL THRESHOLDS

A test run is declared **PASSED** if and only if all of the following conditions are met:
1. **Zero Data Loss**: 100% of injected P0 (input/canonical) and P1 (window) events are persisted.
2. **Zero Plaintext Leakage**: All password fields, credit card numbers, and secret tokens are redacted.
3. **Monotonic Integrity**: Global event IDs and monotonic timestamps are strictly non-decreasing with zero duplicates.
4. **Crash Recovery Bound**: Unflushed data loss on simulated hard kill (`taskkill /F`) is $< 2.0$ seconds.
5. **UIA Timeout Guard**: Slow/hung UIA calls timeout at $100\text{ ms} \pm 10\text{ ms}$ without dropping inputs.
6. **Performance Limits**:
   - Idle CPU $< 1.0\%$
   - Active Recording CPU $< 5.0\%$
   - Private Working Set RAM $< 200\text{ MB}$
   - Input hook processing latency $< 50\mu\text{s}$

---

# 7. HOW TO EXECUTE THE TEST SUITES

```bash
# 1. Run all Tier 1 Feature Coverage Tests
cargo test -p tier1-feature -- --nocapture

# 2. Run E2E Test Runner Verification Modules
cargo test -p e2e-runner -- --nocapture

# 3. Run Specific Feature Suite (e.g. Privacy Redaction)
cargo test -p tier1-feature --test test_f18_privacy_redaction -- --nocapture

# 4. Run Test Harness Application in Standalone Mode
cargo run -p harness-app -- --mode interactive

# 5. Run Full 30-Minute Cross-App E2E Scenario
cargo test -p e2e-runner --test test_30min_workflow -- --nocapture
```
