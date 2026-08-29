# Trajectory Recorder: Data Schema & Storage Specifications

**Schema Identifier**: `gtf.trajectory`  
**Current Schema Version**: `1.0`  
**Document Classification**: Canonical Production Data Schema Specification  

---

## Table of Contents
1. [Dual Timestamp & Clock Synchronization](#1-dual-timestamp--clock-synchronization)
2. [Global Identifiers & Partition Naming](#2-global-identifiers--partition-naming)
3. [Raw Event Schema (`events.raw.ndjson`)](#3-raw-event-schema-eventsrawndjson)
4. [Canonical Action Schema (`events.normalized.ndjson`)](#4-canonical-action-schema-eventsnormalizedndjson)
5. [Session Directory Layout & Manifests](#5-session-directory-layout--manifests)
6. [SQLite WAL Relational Schema (`session.db`)](#6-sqlite-wal-relational-schema-sessiondb)
7. [PostgreSQL Ingestion Cluster Schema](#7-postgresql-ingestion-cluster-schema)
8. [Object Store Key Structure & Archive Formats](#8-object-store-key-structure--archive-formats)

---

## 1. Dual Timestamp & Clock Synchronization

Every recorded event and canonical action contains a high-precision `DualTimestamp` combining human-readable UTC wall time with sub-microsecond monotonic clock nanoseconds and local timezone offsets.

### DualTimestamp JSON Definition
```json
{
  "wall_time_utc": "2026-08-29T03:45:12.123456789Z",
  "monotonic_ns": 4829104928172,
  "timezone_offset_secs": 25200
}
```

### Rust Structure (`crates/core-types/src/timestamp.rs`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DualTimestamp {
    /// UTC wall-clock time in RFC 3339 format for indexing and search.
    pub wall_time_utc: chrono::DateTime<chrono::Utc>,
    /// High-resolution monotonic nanoseconds (QPC/Instant) for interval calculations.
    pub monotonic_ns: u64,
    /// Local machine timezone offset from UTC in seconds at capture time.
    pub timezone_offset_secs: i32,
}
```

### Monotonic Synchronization Rules
- **Wall Time**: Sourced from system UTC clock (`GetSystemTimePreciseAsFileTime` / `chrono::Utc::now()`). Subject to NTP adjustments.
- **Monotonic Nanoseconds**: Sourced from QueryPerformanceCounter (QPC) or `std::time::Instant`. Strictly monotonic and immune to system clock slews, time jumps, and leap seconds.
- **Elapsed Duration**: Time deltas between events are calculated using monotonic nanoseconds:
  $$\Delta t = t_2.\text{monotonic\_ns} - t_1.\text{monotonic\_ns}$$

---

## 2. Global Identifiers & Partition Naming

### 2.1 Global Event ID (`GlobalEventId`)
- A 64-bit strictly monotonic integer assigned sequentially to every captured event across all sessions on a workstation.
- Persisted crash-safely using pre-allocated blocks of 10,000 IDs in `{spool_root}/global_event_id.dat`.

### 2.2 Session Identifier (`SessionId`)
- Generated uniquely for every 1-hour recording partition.
- Format: `{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}`
- Example: `PC-OFFICE-01_20260829_080000_a1b2c3d4`

---

## 3. Raw Event Schema (`events.raw.ndjson`)

Raw events capture unaggregated hardware hooks and telemetry streams post-privacy filtering. Each line in `events.raw.ndjson` is a single JSON object adhering to `RawEvent`.

### Top-Level `RawEvent` Envelope
```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0",
  "event_id": 1001,
  "global_event_id": 4829102,
  "timestamp": {
    "wall_time_utc": "2026-08-29T03:45:12.100Z",
    "monotonic_ns": 1204928100,
    "timezone_offset_secs": 25200
  },
  "machine_id": "WS-FIN-094",
  "windows_session_id": 1,
  "user_id": "alice.smith",
  "source": "WIN32_HOOK",
  "source_sequence": 5420,
  "payload": {
    "kind": "mouse",
    "data": { ... }
  }
}
```

### Event Sources (`EventSource`)
- `WIN32_HOOK`: Low-level mouse or keyboard hook.
- `INPUT_HOOK`: Direct input injection capture.
- `WIN_EVENT`: Foreground window lifecycle event (`SetWinEventHook`).
- `UI_AUTOMATION`: Accessibility tree element inspection.
- `BROWSER_EXTENSION`: Manifest V3 browser companion event.
- `CLIPBOARD_LISTENER`: Clipboard format change notification.
- `FILE_WATCHER`: File system change notification (`ReadDirectoryChangesW`).
- `WGC_SCREEN_CAPTURE`: DirectX / GDI screen capture.
- `SYSTEM_TELEMETRY`: Workstation power, sleep, lock, or logon state.
- `SESSION_ROUTER`: Session rotation or boundary signal.

---

### 3.1 Raw Event Payload Variants (`RawEventPayload`)

#### 1. Mouse Event (`RawMouseEvent`)
```json
{
  "kind": "mouse",
  "data": {
    "event_type": "MOUSE_DOWN",
    "button": "LEFT",
    "coords": {
      "physical_x": 1280,
      "physical_y": 720,
      "normalized_x": 0.6666,
      "normalized_y": 0.6666
    },
    "monitor_id": 0,
    "delta_x": 0.0,
    "delta_y": 0.0,
    "state": "PRESSED",
    "physical_x": 1280,
    "physical_y": 720,
    "normalized_x": 0.6666,
    "normalized_y": 0.6666
  }
}
```

#### 2. Keyboard Event (`RawKeyboardEvent`)
```json
{
  "kind": "keyboard",
  "data": {
    "event_type": "KEY_DOWN",
    "vk_code": 83,
    "scan_code": 31,
    "key_name": "S",
    "modifiers": {
      "ctrl": true,
      "alt": false,
      "shift": false,
      "win": false,
      "caps_lock": false,
      "num_lock": true
    },
    "is_injected": false
  }
}
```

#### 3. Window Event (`RawWindowEvent`)
```json
{
  "kind": "window",
  "data": {
    "event_type": "FOREGROUND",
    "hwnd": 1402928,
    "pid": 8920,
    "process_name": "EXCEL.EXE",
    "window_title": "Q3_Financial_Forecast.xlsx - Excel",
    "bounds": {
      "left": 0,
      "top": 0,
      "right": 1920,
      "bottom": 1080,
      "width": 1920,
      "height": 1080
    },
    "monitor_id": 0,
    "dpi": 96
  }
}
```

#### 4. UI Automation Event (`RawUiaEvent`)
```json
{
  "kind": "ui_automation",
  "data": {
    "event_type": "ELEMENT_HOVER",
    "control_type": "UIA_ButtonControlTypeId",
    "name": "Save",
    "automation_id": "btn_ribbon_save",
    "class_name": "NetUIRibbonButton",
    "framework_id": "WPF",
    "bounds": {
      "left": 120,
      "top": 45,
      "right": 160,
      "bottom": 75,
      "width": 40,
      "height": 30
    },
    "is_password": false
  }
}
```

#### 5. Browser DOM Event (`RawBrowserEvent`)
```json
{
  "kind": "browser",
  "data": {
    "tab_id": 42,
    "event_type": "click",
    "url": "https://erp.company.internal/invoices/new",
    "tag_name": "BUTTON",
    "target_id": "submit-btn",
    "target_class": "btn btn-primary px-4",
    "target_text": "Submit Invoice",
    "css_selector": "button#submit-btn.btn-primary",
    "xpath": "//button[@id='submit-btn']",
    "bounds": {
      "left": 450,
      "top": 600,
      "right": 580,
      "bottom": 640,
      "width": 130,
      "height": 40
    }
  }
}
```

#### 6. Clipboard Event (`RawClipboardEvent`)
```json
{
  "kind": "clipboard",
  "data": {
    "format": "CF_UNICODETEXT",
    "byte_length": 128,
    "hash_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "source_hwnd": 1402928
  }
}
```

#### 7. File System Event (`RawFileEvent`)
```json
{
  "kind": "file",
  "data": {
    "action": "MODIFIED",
    "file_path": "C:\\Users\\Alice\\Documents\\Reports\\August_Monthly.docx",
    "old_file_path": null
  }
}
```

#### 8. Screen Topology Event (`RawScreenEvent`)
```json
{
  "kind": "screen",
  "data": {
    "event_type": "SCREENSHOT_CAPTURED",
    "monitor_id": 0,
    "screenshot_file": "screenshots/0000001001_before.webp",
    "diff_ratio": 0.042
  }
}
```

#### 9. System State Event (`RawSystemEvent`)
```json
{
  "kind": "system",
  "data": {
    "event_type": "UNLOCK",
    "details": "User unlocked workstation"
  }
}
```

#### 10. Session Boundary Event (`RawSessionEvent`)
```json
{
  "kind": "session",
  "data": {
    "event_type": "SESSION_ROTATE",
    "session_id": "WS-FIN-094_20260829_090000_f5e4d3c2",
    "reason": "HOURLY_BOUNDARY"
  }
}
```

---

## 4. Canonical Action Schema (`events.normalized.ndjson`)

`CanonicalAction` represents a synthesized, high-level user action (e.g. `CLICK`, `TYPE_TEXT`, `SHORTCUT`, `FILE_OPEN`) enriched with full UI and window context, before/after visual snapshots, and evidence references.

### Complete `CanonicalAction` JSON Structure
```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0",
  "global_event_id": 4829105,
  "session_id": "WS-FIN-094_20260829_080000_a1b2c3d4",
  "session_event_id": 42,
  "timestamp": {
    "wall_time_utc": "2026-08-29T03:45:14.500Z",
    "monotonic_ns": 1206928500,
    "timezone_offset_secs": 25200
  },
  "action_type": "CLICK",
  "confidence": 1.0,
  "target": {
    "name": "Submit Invoice",
    "control_type": "UIA_ButtonControlTypeId",
    "automation_id": "btn_submit_inv",
    "class_name": "WpfButton",
    "framework_id": "WPF",
    "bounding_rect": {
      "left": 450,
      "top": 600,
      "right": 580,
      "bottom": 640,
      "width": 130,
      "height": 40
    },
    "bounding_box": {
      "x": 450,
      "y": 600,
      "width": 130,
      "height": 40
    },
    "is_enabled": true,
    "is_keyboard_focusable": true,
    "is_password": false,
    "value": null,
    "help_text": "Submits invoice to accounting",
    "ancestor_chain": [
      {
        "level": 1,
        "name": "InvoiceFormPanel",
        "control_type": "UIA_PaneControlTypeId",
        "automation_id": "pnl_form",
        "class_name": "StackPanel",
        "framework_id": "WPF"
      },
      {
        "level": 2,
        "name": "InvoiceWindow",
        "control_type": "UIA_WindowControlTypeId",
        "automation_id": "win_invoice",
        "class_name": "NavigationWindow",
        "framework_id": "WPF"
      }
    ],
    "dom_selector": {
      "tag": "BUTTON",
      "role": "button",
      "visible_text": "Submit Invoice",
      "aria_label": "Submit this invoice",
      "id": "btn_submit_inv",
      "class": "btn btn-primary",
      "href": null,
      "placeholder": null,
      "input_type": null,
      "css_selector": "button#btn_submit_inv",
      "xpath": "//button[@id='btn_submit_inv']"
    },
    "xpath": "//button[@id='btn_submit_inv']"
  },
  "context": {
    "application": {
      "process_name": "ERPClient.exe",
      "pid": 8920,
      "executable_path": "C:\\Program Files\\Enterprise\\ERPClient.exe",
      "app_id": "Enterprise.ERP.Client",
      "is_elevated": false
    },
    "window": {
      "hwnd": 1402928,
      "title": "ERP Client - New Invoice #49281",
      "bounds": {
        "left": 100,
        "top": 100,
        "right": 1700,
        "bottom": 950,
        "width": 1600,
        "height": 850
      },
      "is_maximized": false,
      "is_minimized": false,
      "is_foreground": true,
      "is_fullscreen": false,
      "dpi": 96
    },
    "browser": {
      "browser_family": "Chrome",
      "tab_id": 42,
      "url": "https://erp.company.internal/invoices/new",
      "page_title": "ERP Portal - Create Invoice",
      "domain": "erp.company.internal"
    },
    "display": {
      "active_monitor_id": 0,
      "monitor_count": 2,
      "primary_resolution_width": 1920,
      "primary_resolution_height": 1080,
      "virtual_screen_bounds": {
        "left": 0,
        "top": 0,
        "right": 3840,
        "bottom": 1080,
        "width": 3840,
        "height": 1080
      }
    },
    "user_id": "alice.smith",
    "machine_id": "WS-FIN-094",
    "process_name": "ERPClient.exe",
    "process_id": 8920,
    "executable_path": "C:\\Program Files\\Enterprise\\ERPClient.exe",
    "window_title": "ERP Client - New Invoice #49281",
    "window_handle": 1402928,
    "monitor_id": 0,
    "is_fullscreen": false,
    "is_elevated": false
  },
  "before": {
    "screenshot": {
      "file_name": "0000000042_before.webp",
      "relative_path": "screenshots/0000000042_before.webp",
      "sha256": "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
      "width": 1920,
      "height": 1080,
      "format": "image/webp",
      "trigger": "BEFORE_ACTION"
    },
    "ui_state": {
      "focused_element_name": "Submit Invoice",
      "focused_control_type": "UIA_ButtonControlTypeId",
      "focused_automation_id": "btn_submit_inv",
      "modal_active": false,
      "progress_indicator_active": false
    },
    "active_window": null
  },
  "parameters": {
    "kind": "click",
    "detail": {
      "button": "LEFT",
      "click_count": 1,
      "physical_coords": {
        "physical_x": 515,
        "physical_y": 620,
        "normalized_x": 0.2682,
        "normalized_y": 0.5740
      },
      "normalized_coords": {
        "physical_x": 515,
        "physical_y": 620,
        "normalized_x": 0.2682,
        "normalized_y": 0.5740
      },
      "monitor_id": 0
    }
  },
  "after": {
    "screenshot": {
      "file_name": "0000000042_after.webp",
      "relative_path": "screenshots/0000000042_after.webp",
      "sha256": "8f3b2044355508dd81e0568d74f923a215d11a85745b6e9729b323ecbb040e22",
      "width": 1920,
      "height": 1080,
      "format": "image/webp",
      "trigger": "STABILIZED_AFTER_500_MS"
    },
    "ui_state": {
      "focused_element_name": "Success Dialog",
      "focused_control_type": "UIA_WindowControlTypeId",
      "focused_automation_id": "dlg_success",
      "modal_active": true,
      "progress_indicator_active": false
    },
    "active_window": null
  },
  "evidence": {
    "raw_event_ids": [1001, 1002, 1003],
    "video_ranges": [],
    "screenshot_refs": [],
    "state_changes": [
      {
        "kind": "MODAL_APPEARED",
        "description": "Invoice created successfully dialog appeared",
        "target": "dlg_success",
        "details": { "dialog_id": "dlg_success", "message": "Invoice #49281 saved." }
      }
    ]
  },
  "state_evidence": null,
  "duration_ms": 35
}
```

---

### 4.1 Taxonomy of Canonical Action Types (`ActionType`)

| Category | Enum Variant | Description | Parameters Payload |
|---|---|---|---|
| **Mouse Interaction** | `CLICK` | Single click of primary or secondary button | `ClickParams` |
| | `DOUBLE_CLICK` | Rapid double click | `ClickParams` |
| | `RIGHT_CLICK` | Context menu right click | `ClickParams` |
| | `MIDDLE_CLICK` | Middle / wheel click | `ClickParams` |
| | `DRAG_DROP` | Mouse drag gesture exceeding threshold (>5px) | `DragDropParams` |
| | `SCROLL` | Mouse wheel scroll gesture with momentum grouping | `ScrollParams` |
| **Keyboard Interaction** | `TYPE_TEXT` | Aggregated typing burst (500ms debounce) | `TypeTextParams` |
| | `KEY_PRESS` | Isolated special key press (Enter, Esc, F-keys) | `KeyPressParams` |
| | `SHORTCUT` | Keyboard shortcut combination (e.g. `Ctrl+S`, `Ctrl+C`) | `ShortcutParams` |
| **Clipboard** | `COPY` | Text or binary copied to clipboard | `ClipboardParams` |
| | `CUT` | Content cut from UI field | `ClipboardParams` |
| | `PASTE` | Content pasted into UI field | `ClipboardParams` |
| **Window Lifecycle** | `WINDOW_SWITCH` | User switched foreground focus to another window | `WindowLifecycleParams` |
| | `WINDOW_OPEN` | New application window created and rendered | `WindowLifecycleParams` |
| | `WINDOW_CLOSE` | Application window closed | `WindowLifecycleParams` |
| **Application Lifecycle** | `APP_OPEN` | Process launched and started message loop | `WindowLifecycleParams` |
| | `APP_CLOSE` | Process terminated | `WindowLifecycleParams` |
| **Browser Navigation** | `NAVIGATE` | URL navigation or SPA page transition | `NavigationParams` |
| **File Operations** | `FILE_OPEN` | File opened via explorer or application | `FileOperationParams` |
| | `FILE_SAVE` | Existing file saved | `FileOperationParams` |
| | `FILE_SAVE_AS` | File saved under new path | `FileOperationParams` |
| | `FILE_CREATE` | New file created in watched directory | `FileOperationParams` |
| | `FILE_COPY` | File duplicated | `FileOperationParams` |
| | `FILE_MOVE` | File moved to new path | `FileOperationParams` |
| | `FILE_RENAME` | File renamed | `FileOperationParams` |
| | `FILE_DELETE` | File deleted | `FileOperationParams` |
| | `FILE_UPLOAD` | File attached/uploaded in web or app form | `FileOperationParams` |
| | `FILE_DOWNLOAD` | File downloaded from browser or network | `FileOperationParams` |
| | `FILE_EXPORT` | Data exported to PDF/CSV/XLSX | `FileOperationParams` |
| **Dialog Interaction** | `DIALOG_OPEN` | Common dialog (Open/Save/Print) appeared | `DialogParams` |
| | `DIALOG_CONFIRM`| User clicked OK/Save/Yes on dialog | `DialogParams` |
| | `DIALOG_CANCEL` | User clicked Cancel/Close/No on dialog | `DialogParams` |
| **System & Workstation** | `WAIT` | System busy / loading spinner active (>1.0s) | `WaitParams` |
| | `USER_IDLE` | User inactive exceeding idle threshold (>60s) | `SystemStateParams` |
| | `SYSTEM_LOCK` | Workstation locked (Win+L / screensaver) | `SystemStateParams` |
| | `SYSTEM_UNLOCK` | Workstation unlocked | `SystemStateParams` |
| | `SYSTEM_SLEEP` | System entered ACPI sleep / hibernate | `SystemStateParams` |
| | `SYSTEM_RESUME` | System resumed from sleep | `SystemStateParams` |
| **Fallback** | `UNKNOWN_INTERACTION` | Uncategorized low-level interaction | `UnknownParams` |

---

## 5. Session Directory Layout & Manifests

### 5.1 Directory Layout
```
spool/recording/WS-FIN-094_20260829_080000_a1b2c3d4/
├── manifest.json                 # Session lifecycle and integrity metadata
├── events.raw.ndjson             # Append-only raw event stream
├── events.normalized.ndjson      # Canonical actions stream
├── session.db                    # Embedded SQLite WAL database
├── session.db-wal
├── session.db-shm
├── screenshots/                  # WebP image captures
│   ├── 0000000042_before.webp
│   └── 0000000042_after.webp
├── video/                        # Video segments
├── browser/                      # Browser DOM traces
├── uia/                          # UIA subtree snapshots
└── diagnostics/                  # Performance counters & logs
```

### 5.2 Session Manifest Schema (`manifest.json`)
```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0",
  "session_id": "WS-FIN-094_20260829_080000_a1b2c3d4",
  "machine_id": "WS-FIN-094",
  "user_id": "alice.smith",
  "started_at": "2026-08-29T08:00:00.000000000Z",
  "ended_at": "2026-08-29T08:59:59.999999999Z",
  "status": "FINALIZED",
  "event_count": 14920,
  "action_count": 842
}
```

### 5.3 Staged Chunk Manifest (`manifest.json` inside chunks directory)
```json
{
  "session_id": "WS-FIN-094_20260829_080000_a1b2c3d4",
  "created_at_utc": "2026-08-29T09:00:02.124Z",
  "uncompressed_size_bytes": 184592010,
  "compressed_size_bytes": 74829104,
  "is_encrypted": true,
  "archive_sha256": "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
  "chunk_count": 2,
  "chunk_size_bytes": 67108864,
  "chunks": [
    {
      "chunk_index": 0,
      "file_name": "chunk_0000.bin",
      "byte_size": 67108864,
      "sha256": "d8e8fca2dc0f896fd7cb4cb0031ba249"
    },
    {
      "chunk_index": 1,
      "file_name": "chunk_0001.bin",
      "byte_size": 7720240,
      "sha256": "8a359218204910ef8a1098412850912f"
    }
  ],
  "file_list": [
    "manifest.json",
    "events.raw.ndjson",
    "events.normalized.ndjson",
    "session.db",
    "screenshots/0000000042_before.webp",
    "screenshots/0000000042_after.webp"
  ]
}
```

---

## 6. SQLite WAL Relational Schema (`session.db`)

Every hourly session partition embeds an SQLite database (`session.db`) configured with `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;`.

```sql
CREATE TABLE IF NOT EXISTS session_meta (
    session_id TEXT PRIMARY KEY,
    machine_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    start_time_utc TEXT NOT NULL,
    start_monotonic_ns INTEGER NOT NULL,
    end_time_utc TEXT,
    end_monotonic_ns INTEGER,
    status TEXT NOT NULL,
    total_events INTEGER DEFAULT 0,
    total_actions INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS raw_events (
    global_event_id INTEGER PRIMARY KEY,
    session_event_id INTEGER NOT NULL,
    timestamp_utc TEXT NOT NULL,
    timestamp_monotonic_ns INTEGER NOT NULL,
    source TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS canonical_actions (
    global_event_id INTEGER PRIMARY KEY,
    session_event_id INTEGER NOT NULL,
    timestamp_utc TEXT NOT NULL,
    timestamp_monotonic_ns INTEGER NOT NULL,
    action_type TEXT NOT NULL,
    confidence REAL NOT NULL,
    target_json TEXT,
    context_json TEXT,
    parameters_json TEXT,
    duration_ms INTEGER
);

CREATE TABLE IF NOT EXISTS screenshots (
    screenshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
    global_event_id INTEGER,
    timestamp_monotonic_ns INTEGER NOT NULL,
    monitor_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    format TEXT NOT NULL,
    byte_size INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS video_segments (
    segment_id INTEGER PRIMARY KEY,
    file_name TEXT NOT NULL,
    start_monotonic_ns INTEGER NOT NULL,
    end_monotonic_ns INTEGER NOT NULL,
    frame_count INTEGER NOT NULL,
    fps INTEGER NOT NULL,
    bitrate_kbps INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS annotations (
    annotation_id INTEGER PRIMARY KEY AUTOINCREMENT,
    global_event_id INTEGER NOT NULL,
    note TEXT NOT NULL,
    tag TEXT,
    created_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS id_allocator (
    key TEXT PRIMARY KEY,
    last_allocated_id INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_actions_type ON canonical_actions(action_type);
CREATE INDEX IF NOT EXISTS idx_actions_ts ON canonical_actions(timestamp_monotonic_ns);
CREATE INDEX IF NOT EXISTS idx_events_ts ON raw_events(timestamp_monotonic_ns);
```

---

## 7. PostgreSQL Ingestion Cluster Schema

The enterprise ingestion server stores machine registration tokens, session upload states, chunk indices, and heartbeat metrics in PostgreSQL 16+.

```sql
-- Machines Table: Registered devices and auth tokens
CREATE TABLE IF NOT EXISTS machines (
    machine_id VARCHAR(64) PRIMARY KEY,
    hostname VARCHAR(255) NOT NULL,
    os_version VARCHAR(255) NOT NULL,
    registration_token VARCHAR(255) NOT NULL,
    registered_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_heartbeat_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(32) DEFAULT 'ACTIVE'
);

-- Sessions Table: Staged and finalized upload sessions
CREATE TABLE IF NOT EXISTS sessions (
    session_id VARCHAR(128) PRIMARY KEY,
    machine_id VARCHAR(64) NOT NULL REFERENCES machines(machine_id),
    user_id VARCHAR(64) NOT NULL,
    start_time_utc TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time_utc TIMESTAMP WITH TIME ZONE,
    status VARCHAR(32) NOT NULL DEFAULT 'INITIATED', -- INITIATED, UPLOADING, ACCEPTED, REJECTED, FAILED
    expected_chunks INTEGER NOT NULL,
    received_chunks INTEGER DEFAULT 0,
    total_size_bytes BIGINT NOT NULL,
    archive_sha256 VARCHAR(64) NOT NULL,
    verified_sha256 BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Session Chunks Table: Object store chunk mapping
CREATE TABLE IF NOT EXISTS session_chunks (
    session_id VARCHAR(128) NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    byte_size INTEGER NOT NULL,
    sha256 VARCHAR(64) NOT NULL,
    storage_key VARCHAR(512) NOT NULL,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, chunk_index)
);

-- Machine Heartbeats Table: Time-series telemetry
CREATE TABLE IF NOT EXISTS machine_heartbeats (
    heartbeat_id BIGSERIAL PRIMARY KEY,
    machine_id VARCHAR(64) NOT NULL REFERENCES machines(machine_id) ON DELETE CASCADE,
    disk_usage_pct REAL NOT NULL,
    active_session_id VARCHAR(128),
    received_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- High-performance query indexes
CREATE INDEX IF NOT EXISTS idx_sessions_machine_id ON sessions(machine_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_chunks_session ON session_chunks(session_id);
CREATE INDEX IF NOT EXISTS idx_heartbeats_machine ON machine_heartbeats(machine_id, received_at DESC);
```

---

## 8. Object Store Key Structure & Archive Formats

### 8.1 S3 / MinIO Storage Key Hierarchy
Encrypted chunks are streamed directly to partitioned S3 bucket keys formatted as:

```
trajectory/{machine_id}/{YYYY}/{MM}/{DD}/{HH}/{session_id}/chunk_{chunk_index:05}.bin
```

Example Key:
```
trajectory/WS-FIN-094/2026/08/29/08/WS-FIN-094_20260829_080000_a1b2c3d4/chunk_00000.bin
```

### 8.2 Binary Chunk Structure (XChaCha20-Poly1305 Ciphertext)
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                   24-byte Random Nonce                        |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|        Encrypted Chunk Payload (TAR.Zstd compressed)          |
|                     (Up to 64 MiB)                            |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|               16-byte Poly1305 Authentication Tag             |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- **Authenticated Additional Data (AAD)**: `{session_id}_chunk_{chunk_index}`
- Any byte tampering or chunk swapping produces an AEAD authentication error and triggers immediate rejection.
