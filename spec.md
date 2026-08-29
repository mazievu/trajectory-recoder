# TRAJECTORY RECORDER — MASTER IMPLEMENTATION SPEC FOR AI CODING AGENT

## 0. VAI TRÒ CỦA AI

Bạn là Senior Rust Engineer + Windows Systems Engineer chịu trách nhiệm xây dựng production software có tên tạm thời:

`Trajectory Recorder`

Không xây prototype hoặc demo.

Mục tiêu là tạo phần mềm Windows chạy tự động trên máy nhân viên, ghi lại quá trình làm việc dưới dạng trajectory có cấu trúc, chia dữ liệu theo giờ và tự động đồng bộ về server công ty.

Phạm vi hiện tại kết thúc tại:

**Capture → Normalize → Store → Upload → Server Storage → Session Viewer**

KHÔNG xây:

* AI tạo Skill
* AI phân tích workflow
* Agent tự thao tác máy
* Workflow automation

Dataset được thiết kế để các hệ thống AI đó có thể sử dụng về sau.

---

# 1. NGUYÊN TẮC KHÔNG ĐƯỢC THAY ĐỔI

Các quyết định dưới đây đã được chốt.

AI KHÔNG được tự ý đổi stack hoặc kiến trúc nếu không có lỗi kỹ thuật khiến implementation không thể thực hiện.

## Stack

Client core:

Rust stable, Edition 2024.

Windows API:

`windows` crate / Win32 API.

Desktop UI:

Tauri 2 + React + TypeScript.

Client local database:

SQLite.

Server:

Rust + Axum.

Server metadata database:

PostgreSQL 16+.

Server binary/object storage:

S3-compatible Object Storage.

Development có thể dùng MinIO.

Serialization:

Serde.

Async runtime:

Tokio.

Compression:

Zstd.

Archive:

TAR + Zstd.

Hash:

SHA-256.

Local encryption:

XChaCha20-Poly1305.

Machine secret/key:

bảo vệ bằng Windows DPAPI.

Browser:

Chrome + Edge extension, Manifest V3.

Browser → Recorder communication:

Chrome Native Messaging Host → Windows Named Pipe.

IPC nội bộ:

Windows Named Pipe.

IPC serialization:

length-prefixed MessagePack bằng `rmp-serde`.

Logging:

`tracing` + `tracing-subscriber`.

Errors:

`thiserror`.

Server database access:

`sqlx`.

Client SQLite:

`rusqlite`.

HTTP client:

`reqwest`.

---

# 2. MỤC TIÊU DỮ LIỆU

Hệ thống phải thu đủ dữ liệu để sau này có thể tái dựng:

```text
Người dùng nhìn thấy gì
        ↓
Người dùng làm gì
        ↓
Tác động vào object nào
        ↓
Ứng dụng phản hồi thế nào
        ↓
State thay đổi ra sao
        ↓
Người dùng tiếp tục làm gì
```

Đơn vị logic chính:

```text
STATE_BEFORE
    ↓
ACTION
    ↓
STATE_AFTER
```

Không coi video là nguồn dữ liệu chính.

Thứ tự ưu tiên:

1. Canonical Action
2. Raw Event
3. Application/Window State
4. UI Automation / DOM
5. State Change
6. Screenshot
7. Video

---

# 3. QUY TẮC RECORD

Recorder hoạt động liên tục.

KHÔNG thực hiện:

```text
Record 1 giờ
→ Stop toàn recorder
→ Upload
→ Start lại
```

Phải thực hiện:

```text
Continuous Capture Stream
           ↓
       Session Router
       ↙           ↘
08:00–09:00     09:00–10:00
```

Boundary session chỉ là logical/storage boundary.

Không được có khoảng trống capture giữa hai session.

---

# 4. KIẾN TRÚC PROCESS

Do Windows Session 0 isolation, Windows Service không trực tiếp capture interactive desktop.

Phải có các process sau.

## 4.1 trajectory-supervisor.exe

Windows Service.

Start khi Windows boot.

Nhiệm vụ:

* machine registration
* health management
* configuration
* session coordination
* uploader supervision
* local spool management
* heartbeat server
* crash recovery
* disk monitoring
* quản lý Capture Agent đang kết nối

Không trực tiếp thực hiện global UI capture trong Session 0.

---

## 4.2 trajectory-agent.exe

Chạy trong interactive Windows user session.

Start tự động ngay khi user login.

Có thể launch bằng:

* Windows Scheduled Task trigger `At log on`

hoặc cơ chế Windows tương đương phù hợp.

Agent chịu trách nhiệm:

* mouse hook
* keyboard interaction
* active window
* UI Automation
* screen capture
* video capture
* clipboard interaction
* user-facing file events
* state monitoring
* browser bridge
* event correlation
* screenshot
* privacy filtering
* gửi event sang Supervisor/Session Writer

Một interactive user session = một Capture Agent.

---

## 4.3 trajectory-uploader.exe

Windows Service hoặc child service được Supervisor quản lý.

Nhiệm vụ:

* finalize archive
* compression
* encryption
* chunking
* upload
* retry
* resume
* checksum verification
* local retention

Network tuyệt đối không nằm trên capture critical path.

---

## 4.4 trajectory-tray.exe

Tauri application.

Nhiệm vụ:

* status
* diagnostics
* session information
* connectivity
* disk usage
* recorder health
* privacy status

Đóng UI không được dừng recorder.

Không cung cấp stealth mode.

---

## 4.5 trajectory-browser-host.exe

Chrome Native Messaging Host viết bằng Rust.

Flow:

```text
Chrome/Edge Extension
        ↓
Native Messaging
        ↓
trajectory-browser-host
        ↓
Named Pipe
        ↓
trajectory-agent
```

---

# 5. REPOSITORY STRUCTURE

Phải dùng Cargo Workspace.

```text
trajectory-recorder/
│
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── ARCHITECTURE.md
├── DATA_SCHEMA.md
├── SECURITY.md
├── DEVELOPMENT.md
│
├── apps/
│   ├── supervisor/
│   ├── capture-agent/
│   ├── uploader/
│   ├── browser-host/
│   ├── server/
│   └── desktop-ui/
│
├── crates/
│   ├── core-types/
│   ├── config/
│   ├── ipc/
│   ├── event-bus/
│   ├── input-win/
│   ├── window-win/
│   ├── uia-win/
│   ├── capture-win/
│   ├── browser-events/
│   ├── clipboard-win/
│   ├── file-events-win/
│   ├── privacy/
│   ├── correlator/
│   ├── session/
│   ├── spool/
│   ├── archive/
│   ├── crypto/
│   ├── upload-client/
│   ├── diagnostics/
│   └── test-support/
│
├── browser-extension/
│
├── server/
│   ├── migrations/
│   └── deployment/
│
├── tests/
│   ├── integration/
│   ├── recovery/
│   ├── performance/
│   └── e2e/
│
└── docs/
```

Không tạo circular dependency giữa crates.

`core-types` không phụ thuộc vào Windows-specific crate.

---

# 6. DEPENDENCY RULES

Collector không được gọi trực tiếp:

* database
* filesystem writer
* HTTP
* screenshot compressor
* uploader

Collector chỉ:

```text
Capture
→ tạo RawEvent
→ enqueue
→ return
```

Critical input callback phải cực ngắn.

Architecture:

```text
Collectors
   ↓
Bounded Event Bus
   ↓
Privacy Engine
   ↓
Correlator
   ↓
Session Router
   ↓
Async Writers
```

---

# 7. EVENT CLOCK

Mọi event phải có hai loại timestamp.

```rust
wall_time_utc
monotonic_ns
```

Thêm:

```text
timezone_offset
```

`monotonic_ns` dùng cho correlation.

`wall_time_utc` dùng để tìm kiếm và server storage.

Không correlation event bằng wall clock đơn thuần.

---

# 8. GLOBAL EVENT ID

Event sequence phải tiếp tục xuyên qua hourly session.

Canonical event:

```text
global_event_id
session_id
session_event_id
```

Ví dụ:

```json
{
  "global_event_id": 9728321,
  "session_id": "PC023_20260829_090000",
  "session_event_id": 413
}
```

Global counter phải crash-safe.

Có thể reserve ID range để tránh disk write mỗi event.

---

# 9. SCHEMA VERSION

Mọi persisted object phải có:

```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0"
}
```

Không silent schema change.

Breaking change:

increment major schema version.

---

# 10. CORE RAW EVENT

Định nghĩa Rust enum chuẩn.

Ví dụ:

```rust
enum RawEventKind {
    Mouse,
    Keyboard,
    Window,
    UiAutomation,
    Browser,
    Clipboard,
    File,
    Screen,
    System,
    Session,
}
```

Base metadata:

```text
event_id
monotonic_ns
wall_time_utc
machine_id
windows_session_id
user_id
source
source_sequence
```

Raw ở đây có nghĩa:

**raw event sau Privacy Engine**.

Dữ liệu secret tuyệt đối không được ghi xuống disk dù dưới dạng raw.

---

# 11. CANONICAL ACTION

Canonical schema tối thiểu:

```json
{
  "schema": "gtf.trajectory",
  "schema_version": "1.0",

  "global_event_id": 842,

  "session_id": "PC023_20260829_090000",
  "session_event_id": 413,

  "timestamp": {
    "wall_time_utc": "...",
    "monotonic_ns": 123456789
  },

  "context": {
    "application": {},
    "window": {},
    "browser": null
  },

  "before": {
    "screenshot": null,
    "ui_state": null
  },

  "action": {
    "type": "click",
    "target": {},
    "parameters": {}
  },

  "after": {
    "screenshot": null,
    "ui_state": null,
    "changes": []
  },

  "evidence": {
    "raw_event_ids": [],
    "video_ranges": []
  }
}
```

---

# 12. ACTION TYPES

Canonical action enum tối thiểu:

```text
CLICK
DOUBLE_CLICK
RIGHT_CLICK

TYPE_TEXT
KEY_PRESS
SHORTCUT

SCROLL

DRAG_DROP

COPY
CUT
PASTE

WINDOW_SWITCH
WINDOW_OPEN
WINDOW_CLOSE

APP_OPEN
APP_CLOSE

NAVIGATE

FILE_OPEN
FILE_SAVE
FILE_SAVE_AS
FILE_CREATE
FILE_COPY
FILE_MOVE
FILE_RENAME
FILE_DELETE
FILE_UPLOAD
FILE_DOWNLOAD
FILE_EXPORT

DIALOG_OPEN
DIALOG_CONFIRM
DIALOG_CANCEL

WAIT
USER_IDLE

SYSTEM_LOCK
SYSTEM_UNLOCK
SYSTEM_SLEEP
SYSTEM_RESUME

UNKNOWN_INTERACTION
```

Không ép event không chắc chắn vào action type sai.

Nếu confidence thấp:

`UNKNOWN_INTERACTION`.

---

# 13. MOUSE

Sử dụng Win32 low-level mouse hook phù hợp.

Capture:

* left click
* right click
* middle click
* double click
* mouse down/up
* scroll
* horizontal scroll
* drag/drop

Không persist toàn bộ mouse move.

Mouse movement dùng cho:

* drag path
* interaction context

Coordinate lưu:

```text
physical_x
physical_y
normalized_x
normalized_y
monitor_id
```

Target semantic được ưu tiên hơn coordinate.

---

# 14. KEYBOARD

Không xây raw keylogger lưu mọi printable character.

Phải tách:

### Non-text interaction

Capture:

* modifier
* shortcut
* Enter
* Escape
* arrows
* Delete
* function keys
* Tab
* navigation

### Text input

Printable text chỉ được persist nếu Privacy Engine có đủ thông tin xác định target không sensitive.

Nguồn xác định target:

1. Browser DOM
2. UI Automation
3. focused control metadata

Nếu target:

```text
password
secret
OTP
credential
payment
```

persist:

```json
{
  "type": "TYPE_TEXT",
  "value": "[REDACTED]",
  "length": 14
}
```

Nếu không xác định được độ an toàn của text field:

fail closed.

Lưu:

```json
{
  "value": "[UNOBSERVED_TEXT]",
  "length": 24
}
```

Không persist plaintext.

---

# 15. TEXT GROUPING

Không tạo:

```text
T
e
s
t
```

thành 4 canonical actions.

Gom typing burst thành:

```text
TYPE_TEXT "Test"
```

Boundary khi:

* target thay đổi
* non-text key xảy ra
* timeout
* focus change
* paste
* submit
* window switch

---

# 16. ACTIVE WINDOW

Track:

* HWND
* PID
* process name
* executable path nếu được phép
* window title
* window bounds
* monitor
* minimized/maximized
* foreground status

Capture event:

* foreground changed
* open
* close
* move
* resize
* minimize
* maximize
* restore

---

# 17. UI AUTOMATION

Dùng Microsoft UI Automation.

Target metadata:

```text
ControlType
Name
AutomationId
ClassName
FrameworkId
BoundingRectangle
IsEnabled
IsKeyboardFocusable
IsPassword
Value
HelpText
```

Thêm hierarchy giới hạn:

```text
target
parent
grandparent
relevant ancestors
```

Không serialize toàn bộ UI tree cho mỗi click.

UIA call có timeout.

Nếu UIA treo:

* cancel/abandon request
* mark target unavailable
* không block input pipeline

Fallback:

```text
window
coordinate
screenshot
```

---

# 18. BROWSER EXTENSION

Manifest V3.

Hỗ trợ:

* Chrome
* Edge

Capture:

* URL
* page title
* domain
* tab ID
* navigation
* SPA navigation
* new tab
* close tab
* back
* forward
* reload

Interaction:

* click
* dblclick
* input
* change
* submit
* focus
* blur
* scroll
* selection
* drag/drop
* keyboard interaction

DOM target:

```text
tag
role
visible_text
aria_label
name
id
class
href
placeholder
input_type
bounding_rect
CSS selector
DOM path
XPath fallback
```

Không gửi password value.

Không đọc content của password field.

---

# 19. BROWSER DOM STATE

Không dump full DOM liên tục.

Chỉ capture semantic snapshot khi:

* navigation hoàn thành
* target interaction
* modal xuất hiện
* major DOM mutation
* action result cần evidence

Snapshot phải giới hạn size.

Ưu tiên accessibility/semantic information hơn raw HTML.

---

# 20. SCREENSHOT

Định dạng:

WebP.

Capture tại minimum:

```text
before
after
```

cho interaction quan trọng.

Có thể capture additional delayed state:

```text
+200ms
+500ms
+1000ms
```

nhưng phải tránh tạo ảnh dư thừa.

Implement screen-state stabilization bằng perceptual/screen diff hợp lý.

Không cần AI vision.

---

# 21. CONTINUOUS VIDEO

Video là secondary evidence nhưng bản production phải hỗ trợ.

Default config:

```text
continuous_video = true
fps = 10
codec = H264
hardware_encoding = preferred
```

Dùng Windows Media Foundation hoặc API phù hợp.

Nếu hardware encoding không có:

fallback software encoder nhưng phải tuân theo CPU budget.

Multi-monitor:

ưu tiên video stream riêng cho từng monitor.

Mỗi video fragment phải map được:

```text
monotonic start
monotonic end
```

Canonical event có thể map sang video range.

---

# 22. SCREEN CAPTURE PRIVACY

Không capture:

* Windows lock screen
* secure desktop
* credential UI nếu Windows đánh dấu protected
* application/domain thuộc policy exclusion

Nếu exclusion xảy ra:

timeline vẫn lưu:

```text
PRIVACY_EXCLUDED_RANGE
```

nhưng không lưu pixels/content.

---

# 23. CLIPBOARD

Track action:

* copy
* cut
* paste

Metadata mặc định:

```text
content_type
length
hash
source_app
destination_app
```

Không mặc định lưu clipboard plaintext.

Chỉ lưu plaintext nếu policy explicitly cho phép và Privacy Engine xác nhận safe.

Default:

metadata-only.

---

# 24. FILE OPERATIONS

Không cố log toàn bộ filesystem của Windows.

Mục tiêu là **user workflow file operations**.

Nguồn:

* file dialog UIA
* Explorer interaction
* browser download/upload
* foreground process telemetry
* filesystem watcher ở relevant user paths
* ETW nếu cần để correlate process/file operation

Track:

```text
OPEN
CREATE
SAVE
SAVE_AS
COPY
MOVE
RENAME
DELETE
UPLOAD
DOWNLOAD
EXPORT
```

Metadata:

```text
path
filename
extension
size
source_app
operation
timestamp
```

Không copy file content vào trajectory.

---

# 25. FILE DIALOG

Phải nhận diện Windows common dialogs:

* Open
* Save
* Save As
* Select Folder

Track:

* dialog opened
* selected path
* selected filename
* filter
* confirm
* cancel

Đây là semantic event quan trọng.

---

# 26. DRAG DROP

Normalize sequence:

```text
mouse_down
movement
mouse_up
```

thành:

```text
DRAG_DROP
```

nếu đủ điều kiện.

Metadata:

```text
source_target
destination_target
start
end
duration
path_summary
```

Nếu browser drag/drop:

DOM metadata ưu tiên.

---

# 27. SCROLL

Canonical scroll:

```text
direction
delta
container
duration
start_position
end_position
```

Phân biệt nếu có thể:

* page
* modal
* panel
* spreadsheet
* dropdown

Không tạo hàng trăm canonical scroll actions cho một continuous wheel gesture.

Gom thành scroll burst.

---

# 28. WAIT

Không coi mọi khoảng không input là WAIT.

WAIT được tạo khi có evidence cho thấy user/action đang chờ system.

Ví dụ:

```text
CLICK Generate
→ loading state
→ no input
→ result appears
```

Canonical:

```text
CLICK Generate
WAIT 13.7 sec
RESULT_STATE
```

Nếu chỉ người dùng không thao tác:

`USER_IDLE`.

---

# 29. STATE CHANGE

Thu các state change xác định được:

```text
dialog appeared
dialog disappeared
modal appeared
toast
error
loading started
loading ended
page navigation
window appeared
window disappeared
file created
download completed
```

Nguồn:

* UIA event
* browser MutationObserver
* window event
* filesystem event
* screen change

---

# 30. EVENT CORRELATOR

Đây là component quan trọng nhất sau collector.

Input có thể là:

```text
mouse_down
mouse_up
DOM click
UIA focus
screen change
URL change
```

Không được persist chúng như 6 workflow action độc lập.

Correlator phải tạo:

```text
CLICK "Save"
```

và reference các raw event liên quan.

Correlation dựa trên:

* monotonic timestamp
* active application
* target bounds
* HWND
* browser tab
* DOM target
* UIA target
* event type

Không xóa raw evidence.

---

# 31. CORRELATION CONFIDENCE

Canonical target/action có:

```text
confidence
```

0.0–1.0.

Nguồn semantic priority:

Browser DOM

> UI Automation
> accessibility metadata
> window metadata
> coordinate.

Không fabricate target name.

---

# 32. PRIVACY ENGINE

Privacy Engine nằm trước persistent storage.

Mandatory rules:

* password
* OTP
* credit card
* credential
* API key/secret patterns
* auth token
* excluded app
* excluded domain
* secure Windows UI

Dữ liệu sensitive không được persist plaintext vào:

* JSONL
* SQLite
* logs
* screenshots
* video
* diagnostics

Privacy filter failure phải ưu tiên không lưu hơn lưu secret.

---

# 33. SESSION ROUTER

Session partition theo local clock hour.

Ví dụ máy start lúc:

08:37.

Session:

```text
08:37–09:00
09:00–10:00
10:00–11:00
```

Boundary phải dùng timestamp chính xác.

Session cũ finalize background.

Session mới nhận event ngay.

---

# 34. SESSION ID

Format:

```text
{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}
```

Không dựa duy nhất vào timestamp để tránh collision.

---

# 35. SESSION DIRECTORY

Trong lúc record:

```text
spool/
└── recording/
    └── SESSION_ID/
```

Nội dung:

```text
manifest.json
events.raw.ndjson
events.normalized.ndjson
index.sqlite

screenshots/
video/
browser/
uia/
diagnostics/
```

Sau finalize:

```text
SESSION_ID.tar.zst
```

Sau encryption:

```text
SESSION_ID.trajectory.enc
```

---

# 36. SQLITE

SQLite dùng làm local index/recovery metadata.

Bật:

```text
WAL mode
```

Không dùng SQLite làm nơi duy nhất lưu event stream.

Primary append log:

NDJSON/chunked append files.

Nếu DB corrupt vẫn phải recover event stream.

---

# 37. WRITE STRATEGY

Event writer:

* append-only
* buffered
* periodic flush
* bounded queue

Không flush disk mỗi key/mouse event.

Không chờ Stop mới persist.

Crash phải chỉ mất một khoảng event rất nhỏ.

Target:

< 2 giây dữ liệu chưa flush.

---

# 38. CRASH RECOVERY

Khi Supervisor start:

scan:

```text
spool/recording/
```

Nếu phát hiện unfinished session:

1. kiểm tra manifest
2. validate event chunks
3. truncate partial invalid tail nếu cần
4. rebuild index nếu cần
5. mark:

`RECOVERED`

6. finalize
7. đưa vào pending upload

Không xóa unfinished session.

---

# 39. LOCAL SPOOL STATES

Directories:

```text
spool/
├── recording/
├── finalizing/
├── pending_upload/
├── uploading/
├── uploaded/
└── failed/
```

State transition phải atomic bằng directory rename khi có thể.

---

# 40. ARCHIVE

Finalize:

```text
Session directory
    ↓
TAR
    ↓
Zstd
    ↓
Encrypt
```

Không encrypt từng event riêng.

Archive SHA-256 tính trên encrypted final archive hoặc được định nghĩa rõ và nhất quán.

Lưu hash loại nào trong manifest.

---

# 41. ENCRYPTION

Tạo per-machine encryption key.

Key được bảo vệ bằng DPAPI.

Archive encrypt bằng:

XChaCha20-Poly1305.

Nonce không reuse.

Không hardcode encryption key.

---

# 42. UPLOAD CHUNK

Default:

64 MiB/chunk.

Configurable:

64–256 MiB.

Mỗi chunk có:

```text
session_id
chunk_index
chunk_size
chunk_sha256
```

Upload phải resumable.

---

# 43. SERVER API

Implement versioned API.

## Register machine

```text
POST /api/v1/machines/register
```

Input:

```json
{
  "enrollment_token": "...",
  "machine_name": "...",
  "machine_fingerprint": "...",
  "recorder_version": "..."
}
```

Output:

```json
{
  "machine_id": "...",
  "device_token": "..."
}
```

Store device token using DPAPI.

---

## Heartbeat

```text
POST /api/v1/machines/heartbeat
```

Data:

```text
machine_id
recorder_version
agent_status
uploader_status
disk_free
current_session
pending_sessions
last_event_at
```

---

## Initialize upload

```text
POST /api/v1/sessions
```

Input:

```text
session_id
machine_id
started_at
ended_at
archive_size
archive_sha256
chunk_count
schema_version
```

Must be idempotent.

---

## Upload chunk

```text
PUT /api/v1/sessions/{session_id}/chunks/{index}
```

Headers:

```text
X-Chunk-SHA256
X-Chunk-Size
```

Server verifies hash.

---

## Upload status

```text
GET /api/v1/sessions/{session_id}/upload-status
```

Response includes received chunk indexes.

Client only uploads missing chunks.

---

## Complete

```text
POST /api/v1/sessions/{session_id}/complete
```

Server:

1. verify all chunks
2. reconstruct object
3. verify final SHA-256
4. mark session ACCEPTED

Response:

```json
{
  "status": "SESSION_ACCEPTED"
}
```

Client không delete local session trước response này.

---

# 44. SERVER DATABASE

Minimum tables:

```text
machines
sessions
session_chunks
heartbeats
employees
machine_user_mappings
```

Session unique constraint:

```text
machine_id + session_id
```

Retry không tạo duplicate.

---

# 45. OBJECT STORAGE

Không lưu:

* screenshots
* video
* archive blob

trực tiếp trong PostgreSQL.

PostgreSQL chỉ lưu metadata.

Archive lưu object storage.

Object key:

```text
trajectory/
{machine_id}/
{YYYY}/
{MM}/
{DD}/
{HH}/
{session_id}.trajectory.enc
```

---

# 46. NETWORK FAILURE

Capture không phụ thuộc server.

Nếu mất mạng:

```text
session 09
session 10
session 11
session 12
```

đều tiếp tục lưu local.

Uploader retry exponential backoff có jitter.

Khi mạng trở lại:

upload oldest session trước.

---

# 47. DISK PROTECTION

Implement disk pressure levels.

## Level 0

<70% disk used.

Full capture.

## Level 1

70–85%.

Giảm video bitrate.

## Level 2

85–92%.

Disable continuous video.

Giữ screenshots/event.

## Level 3

> 92%.

Critical trajectory mode.

Ưu tiên:

1. canonical
2. raw input
3. window/app
4. DOM/UIA
5. important screenshots

General screenshot/video có thể drop.

Phải emit health warning.

---

# 48. BACKPRESSURE

Mọi queue phải bounded.

Priority:

```text
P0 canonical/raw input
P1 window/state
P2 DOM/UIA
P3 screenshot
P4 video
```

Nếu overload:

drop P4 trước.

Không drop mouse click/key shortcut để giữ video frame.

---

# 49. LOGOUT

Khi user logout:

Capture Agent:

* stop capture
* flush event queue
* close video segment
* emit USER_LOGOUT
* disconnect Supervisor

Session có thể kết thúc trước hourly boundary.

---

# 50. LOCK SCREEN

Emit:

```text
SYSTEM_LOCK
```

Pause pixels/text capture.

Unlock:

```text
SYSTEM_UNLOCK
```

resume.

Timeline vẫn liên tục.

---

# 51. SLEEP/HIBERNATE

Emit:

```text
SYSTEM_SLEEP
SYSTEM_RESUME
```

Không tính sleep duration thành USER_IDLE.

---

# 52. MULTI-MONITOR

Support:

* monitor hotplug
* resolution change
* DPI scaling
* orientation
* primary screen change

Mỗi event coordinate phải biết monitor.

Screen topology changes phải thành event.

---

# 53. SESSION VIEWER

Xây web/admin viewer cơ bản.

Không cần AI.

Viewer list:

* machine
* employee mapping
* date
* session
* duration
* upload status

Session view:

timeline.

Ví dụ:

```text
09:03 Chrome
09:03 CLICK "Products"
09:04 CLICK "Add product"
09:04 TYPE_TEXT "Custom Pet Nails"
09:05 FILE_UPLOAD dog.png
09:06 CLICK "Save"
09:06 WAIT 1.4s
09:06 Toast "Product saved"
09:08 WINDOW_SWITCH Photoshop
```

Click canonical action hiển thị:

* timestamp
* app
* window
* target
* before screenshot
* after screenshot
* raw evidence
* DOM/UIA metadata
* video timestamp
* confidence

---

# 54. SEARCH

Viewer hỗ trợ:

```text
machine
user
application
domain
action
target text
file extension
date range
errors
upload/download
typing
drag/drop
wait
```

---

# 55. SECURITY

Không implement stealth monitoring.

Recorder phải có visible system status/tray indicator theo policy triển khai.

Không cho process tự disable antivirus/security control.

Không cố bypass Windows security boundary.

Không capture secure desktop.

Không ghi secret trong logs.

---

# 56. CONFIG

Config gồm hai tầng:

```text
machine config
server policy
```

Server policy override các setting security/privacy.

Minimum:

```text
continuous_video
video_fps
video_bitrate
screenshot_mode

excluded_apps
excluded_domains

local_retention_hours

upload_chunk_size

disk_thresholds
```

Config phải versioned.

---

# 57. OBSERVABILITY

Structured logs.

Fields:

```text
process
module
machine_id
session_id
event
severity
```

Metrics:

```text
events/sec
queue depth
event drops
screenshot latency
UIA latency
disk write latency
video encoder FPS
pending upload GB
upload throughput
failed chunks
```

---

# 58. PERFORMANCE TARGET

Không bật continuous video:

```text
Idle CPU            <1%
Normal capture      target <5%
RAM Agent           target <200 MB
Input added latency không cảm nhận được
```

Có video:

CPU phụ thuộc encoder nhưng không được làm máy mất khả năng làm việc bình thường.

Input callback không được:

* await
* network
* disk
* UIA tree traversal
* image compression

---

# 59. THREADING / ASYNC MODEL

Không tạo uncontrolled threads.

Recommended:

```text
Win32 callback
    ↓
lock-free hoặc low-contention bounded queue
    ↓
collector worker
    ↓
Tokio event pipeline
```

UIA có worker riêng vì COM có threading constraints.

Capture/video worker riêng.

Disk writer riêng.

Uploader process riêng.

---

# 60. ERROR TAXONOMY

Định nghĩa error enum theo module.

Phân loại:

```text
Transient
Recoverable
Degraded
Fatal
PrivacyCritical
```

Ví dụ:

Browser extension disconnect:

Degraded.

UIA timeout:

Recoverable.

Disk full:

Degraded → Critical.

Encryption key unavailable:

Fatal.

Privacy filter uncertain:

PrivacyCritical → fail closed.

---

# 61. MODULE FAILURE RULE

Một collector fail không được giết toàn Capture Agent nếu không bắt buộc.

Ví dụ:

```text
UIA fail
→ vẫn mouse + window + screenshot

Browser host fail
→ vẫn screen + input + window

Video fail
→ trajectory vẫn chạy
```

Emit health degradation event.

---

# 62. TEST STRATEGY

Không merge module nếu không có tests tương ứng.

## Unit tests

* serialization
* schema
* session boundary
* text grouping
* scroll grouping
* drag detection
* privacy rules
* correlation
* archive
* checksum
* chunking
* upload resume

## Integration tests

* Named Pipe
* agent/supervisor
* browser-host/agent
* SQLite recovery
* spool transitions
* server upload

## Crash tests

Kill process giữa:

* event write
* session rotation
* compression
* encryption
* chunk upload

Sau restart phải recover.

## Network tests

Simulate:

* timeout
* packet loss
* server unavailable
* failed chunk
* duplicate request

## Disk tests

Simulate disk thresholds.

---

# 63. WINDOWS TEST HARNESS

Tạo một app test riêng để tự động kiểm tra capture.

App test phải có:

* button
* textbox
* password field
* dropdown
* scroll panel
* dialog
* drag/drop
* save file
* progress/loading state

Automated test thực hiện interaction rồi xác nhận trajectory.

Đừng phụ thuộc Photoshop/Office để chạy CI.

---

# 64. MANUAL ACCEPTANCE

Sau automated testing, test thủ công workflow thực tế:

```text
Chrome
→ Excel
→ Explorer
→ Photoshop hoặc ứng dụng desktop tương tự
→ Chrome
```

Thời lượng:

30 phút trở lên.

---

# 65. ACCEPTANCE REQUIREMENT

Một kỹ sư không nhìn người dùng thực hiện workflow phải có thể xem trajectory và xác định:

* app nào được mở
* chuyển app lúc nào
* cửa sổ nào active
* click target nào
* nhập text vào đâu
* shortcut gì
* copy từ đâu
* paste sang đâu
* file nào được chọn
* upload file nào
* download file nào
* drag từ đâu tới đâu
* scroll cái gì
* dialog gì xuất hiện
* hệ thống loading khi nào
* người dùng chờ bao lâu
* result state là gì
* output cuối là gì

Workflow đi qua hourly boundary phải nối lại được.

---

# 66. SESSION BOUNDARY TEST

Bắt đầu workflow:

```text
09:58
```

kết thúc:

```text
10:07
```

Phải xuất hiện ở hai session:

```text
09:00–10:00
10:00–11:00
```

nhưng Global Timeline phải reconstruct thành một chuỗi liên tục.

Zero lost canonical input events tại boundary.

---

# 67. BOOT TEST

Test:

1. Windows boot
2. Supervisor auto start
3. user login
4. Capture Agent auto start
5. capture bắt đầu mà user không cần mở app
6. tray hiển thị recorder running

---

# 68. SERVER DOWN TEST

Tắt server trong 8 giờ.

Client phải:

* tiếp tục record
* tiếp tục rotate session
* giữ pending upload
* không tăng CPU bất thường
* không mất session

Bật server.

Uploader phải upload backlog oldest-first.

---

# 69. DEFINITION OF DONE PRODUCTION

Chỉ coi hệ thống hoàn thành khi toàn bộ điều sau đạt:

1. Boot tự start Supervisor.
2. Login tự start Capture Agent.
3. Không cần user bấm Record.
4. Capture liên tục.
5. Session partition theo giờ.
6. Không có gap giữa session.
7. Global Event ID xuyên session.
8. Mouse capture.
9. Keyboard semantic capture.
10. Sensitive text redacted trước disk.
11. Window tracking.
12. UIA semantic target.
13. Browser DOM target.
14. Clipboard event.
15. Relevant file operation.
16. Drag/drop.
17. Scroll grouping.
18. Wait/state detection.
19. Screenshot evidence.
20. Continuous video.
21. Multi-monitor.
22. Lock/sleep handling.
23. Crash recovery.
24. Local spool.
25. Compression.
26. Encryption.
27. Chunk resumable upload.
28. Server checksum verification.
29. Server idempotency.
30. PostgreSQL metadata.
31. Object storage.
32. Session Viewer.
33. Search/filter.
34. Disk degradation.
35. Network failure recovery.
36. UIA failure degradation.
37. Browser extension failure degradation.
38. Raw trajectory retained.
39. Normalized trajectory regenerate được.
40. E2E acceptance workflow đạt.

---

# 70. IMPLEMENTATION ORDER

Không build ngẫu nhiên.

Thực hiện theo thứ tự sau.

## Phase 1 — Foundation

* Cargo workspace
* core-types
* schema
* timestamp
* config
* diagnostics
* machine identity
* Named Pipe IPC

Phải test xong mới sang Phase 2.

## Phase 2 — Capture Core

* mouse
* keyboard
* active window
* event bus
* raw writer

## Phase 3 — Semantic Capture

* UI Automation
* text grouping
* scroll grouping
* drag/drop
* privacy engine

## Phase 4 — State Evidence

* screenshot
* state change
* screen stabilization
* continuous video

## Phase 5 — Browser

* extension
* native host
* DOM events
* browser state

## Phase 6 — Session Engine

* global event ID
* session routing
* hourly rotation
* append storage
* SQLite index
* crash recovery

## Phase 7 — Upload Pipeline

* spool
* archive
* compression
* encryption
* chunking
* retry

## Phase 8 — Server

* machine registration
* heartbeat
* session API
* chunk API
* PostgreSQL
* object storage
* idempotency

## Phase 9 — Viewer

* sessions
* timeline
* event inspector
* screenshots
* video mapping
* filters

## Phase 10 — Hardening

* performance
* disk pressure
* network failure
* crash testing
* multi-monitor
* deployment
* installer
* auto-update strategy

---

# 71. AI CODING RULES

Khi thực hiện spec này:

### Phải

* đọc toàn bộ architecture trước khi sửa module
* giữ module boundaries
* viết code production-quality
* viết tests
* xử lý error thật
* logging structured
* cleanup resource
* đóng Windows handle đúng cách
* dùng RAII
* document unsafe block
* minimize unsafe code
* giải thích invariant của unsafe code

### Không được

* thay Rust bằng Python/Node
* dùng Electron thay Tauri
* nhét recorder vào UI
* để UI đóng là recorder chết
* dùng HTTP trên input callback
* ghi DB trên input callback
* lưu raw plaintext password
* hardcode server URL/token/key
* bỏ crash recovery
* bỏ test để “làm sau”
* đổi schema tùy ý
* tạo fake implementation
* TODO cho feature thuộc Definition of Done rồi tuyên bố hoàn thành
* silently ignore errors
* dùng `unwrap()` bừa bãi trong production path

---

# 72. QUY TẮC KHI GẶP ĐIỂM CHƯA RÕ

Nếu có implementation detail chưa được quy định:

1. giữ nguyên architecture
2. chọn giải pháp đơn giản nhất đáp ứng production requirement
3. ghi quyết định vào `ARCHITECTURE.md`
4. không thay đổi product behavior
5. không hỏi lại nếu có thể tự quyết định an toàn

Nếu phát hiện requirement technically impossible hoặc mâu thuẫn:

KHÔNG workaround âm thầm.

Phải:

* ghi rõ vấn đề
* nêu nguyên nhân kỹ thuật
* đề xuất thay đổi nhỏ nhất
* giữ nguyên mục tiêu sản phẩm.

---

# 73. DOCUMENTATION BẮT BUỘC

Repo phải luôn có và cập nhật:

`ARCHITECTURE.md`

Mô tả component/process/data flow.

`DATA_SCHEMA.md`

Mô tả toàn bộ persisted schema.

`SECURITY.md`

Mô tả privacy, encryption, credential handling.

`DEVELOPMENT.md`

Build/run/test.

`README.md`

Setup tổng quan.

Không để documentation lệch code.

---

# 74. OUTPUT CỦA AI SAU MỖI PHASE

Sau mỗi Phase phải báo:

```text
Implemented
Tests
Known limitations
Architecture decisions
Files changed
How to run
How to verify
Next phase
```

Không chỉ báo:

“Done”.

---

# 75. MỤC TIÊU CUỐI

Sản phẩm cuối phải hoạt động như sau:

```text
Windows Boot
      ↓
Supervisor Service
      ↓
User Login
      ↓
Capture Agent
      ↓
Continuous Capture
      ↓
Mouse / Keyboard / Window
UIA / Browser / Screen
File / Clipboard / System
      ↓
Privacy Engine
      ↓
Raw Event Bus
      ↓
State Correlator
      ↓
Canonical Timeline
      ↓
Session Router
      ↓
Hourly Session
      ↓
Local Spool
      ↓
Finalize Background
      ↓
Compress
      ↓
Encrypt
      ↓
Chunk
      ↓
Uploader
      ↓
Company Server
      ↓
PostgreSQL Metadata
+
Object Storage
      ↓
Session Viewer
```

Hệ thống phải tạo ra một dataset đủ chi tiết để sau này một hệ thống AI khác có thể học:

**“Người này đã thực hiện công việc này trên máy tính như thế nào?”**

mà không cần thiết kế lại recorder hoặc yêu cầu nhân viên ghi lại workflow lần nữa.
