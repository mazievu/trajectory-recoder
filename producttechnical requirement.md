zzzzzzzzzzzzzzzzzzzz# YÊU CẦU PHẦN MỀM — TRAJECTORY RECORDER

## 1. Mục tiêu

Xây dựng phần mềm chạy nền trên máy tính nhân viên để ghi lại đầy đủ quá trình thao tác làm việc dưới dạng dữ liệu có cấu trúc.

Mục tiêu không phải chỉ để quay màn hình, mà để tái dựng được:

Người dùng nhìn thấy gì
→ thao tác gì
→ thao tác vào đối tượng nào
→ ứng dụng phản hồi ra sao
→ trạng thái hệ thống thay đổi như thế nào
→ workflow tạo ra kết quả gì.

Dữ liệu này sẽ được dùng về sau cho:

* phân tích workflow,
* xây SOP,
* debug quy trình,
* đào tạo nhân sự,
* tìm thao tác lặp lại,
* tạo dataset cho AI,
* học workflow và tạo Skill/Agent.

Phiên bản hiện tại chỉ chịu trách nhiệm:

**Thu thập → chuẩn hóa → lưu → đồng bộ lên server.**

Chưa bao gồm AI tạo Skill.

---

# 2. Nguyên tắc kiến trúc

Hệ thống phải tuân thủ các nguyên tắc sau.

## 2.1 Record liên tục

Recorder không chạy theo mô hình:

Start
→ Record
→ Stop
→ Upload
→ Start lại.

Recorder phải chạy liên tục.

Việc chia dữ liệu thành từng giờ chỉ là partition ở tầng lưu trữ.

Luồng đúng:

Windows boot
→ Recorder Service tự chạy
→ người dùng login
→ bắt đầu capture desktop
→ capture liên tục
→ Session Router chia dữ liệu theo giờ
→ session cũ được finalize
→ session mới tiếp tục ngay lập tức
→ session cũ được upload nền.

Không được tồn tại khoảng trống capture giữa hai session.

---

# 3. Stack kỹ thuật

## Recorder Core

Rust.

## Windows Integration

Win32 API thông qua `windows-rs`.

## Global Input

Win32 Hooks.

## UI Inspection

Microsoft UI Automation.

## Screen Capture

Windows Graphics Capture.

Có thể dùng Desktop Duplication API cho trường hợp cần thiết.

## Browser Capture

Chrome/Edge Extension.

## Desktop UI

Tauri + React.

## Local Database

SQLite.

## Raw Events

JSONL.

## Screenshot

WebP.

## Video

MP4.

Video là dữ liệu phụ, không phải dữ liệu chính.

## Serialization

Serde.

## Compression

Zstd.

## Archive

ZIP hoặc custom trajectory package.

## IPC

Named Pipe hoặc IPC nội bộ giữa các service.

---

# 4. Kiến trúc process

Hệ thống tối thiểu gồm 3 thành phần độc lập.

## 4.1 Recorder Service

Windows Service.

Chịu trách nhiệm:

* Input Capture
* Window Capture
* UI Automation
* Screen Capture
* Browser Event Capture
* Clipboard Event Capture
* File Event Capture
* State Correlation
* Privacy Filtering
* Session Routing
* Local Writing

Recorder Service phải hoạt động độc lập với giao diện.

Nếu Tauri UI bị đóng hoặc crash, recorder vẫn tiếp tục hoạt động.

---

## 4.2 Uploader Service

Chịu trách nhiệm:

* finalize session,
* compress,
* encrypt,
* chia chunk,
* upload,
* retry,
* kiểm tra checksum,
* quản lý local spool.

Network lỗi không được ảnh hưởng tới recorder.

---

## 4.3 Tauri UI

Chỉ dùng cho:

* trạng thái recorder,
* trạng thái server,
* dung lượng ổ đĩa,
* các session pending,
* diagnostics,
* cấu hình được phép chỉnh,
* Privacy/Policy status.

Đóng UI không được dừng Recorder Service.

---

# 5. Auto Start

Recorder Core phải được cài dưới dạng Windows Service.

Luồng:

Windows Boot
→ Service Control Manager
→ Recorder Service chạy.

Không phụ thuộc Startup Folder.

Không phụ thuộc việc người dùng mở ứng dụng.

Khi chưa login interactive desktop:

* service vẫn hoạt động,
* chưa capture nội dung desktop người dùng.

Sau khi người dùng login:

* bắt đầu capture session.

---

# 6. Session

Dữ liệu được chia thành session theo giờ đồng hồ.

Ví dụ:

08:00–09:00
09:00–10:00
10:00–11:00.

Nếu máy được bật lúc 08:37:

Session đầu:

08:37–09:00.

Sau đó:

09:00–10:00
10:00–11:00.

Không cần session đầu đủ 60 phút.

---

# 7. Global Timeline

Mặc dù chia session theo giờ, event sequence phải liên tục xuyên session.

Mỗi event có:

* global_event_id,
* session_id,
* session_event_id,
* timestamp.

Ví dụ:

global_event_id: 9,728,321
session_id: PC023_20260829_090000
session_event_id: 413.

Nhờ đó workflow bắt đầu 09:58 và kết thúc 10:07 vẫn có thể nối lại chính xác.

---

# 8. Session Metadata

Mỗi session phải có:

* session_id,
* employee_id nếu có,
* machine_id,
* Windows user,
* started_at,
* ended_at,
* timezone,
* OS version,
* recorder version,
* capture configuration,
* screen configuration,
* applications used,
* event count,
* screenshot count,
* video duration,
* compressed size,
* checksum,
* upload status.

Session status:

RECORDING
FINALIZING
PENDING_UPLOAD
UPLOADING
UPLOADED
FAILED
RECOVERED.

---

# 9. Mouse Tracking

Ghi lại các interaction có ý nghĩa:

* left click,
* right click,
* middle click,
* double click,
* mouse down,
* mouse up,
* drag start,
* drag path,
* drag end,
* scroll vertical,
* scroll horizontal.

Không lưu toàn bộ mouse movement từng pixel.

Mouse movement được sampling hoặc simplification thành trajectory khi có ý nghĩa.

Mỗi event mouse lưu:

* x,
* y,
* normalized_x,
* normalized_y,
* monitor,
* button,
* duration,
* target nếu xác định được.

Tọa độ chỉ là fallback.

Target semantic mới là dữ liệu ưu tiên.

---

# 10. Keyboard Tracking

Track:

* key down/up,
* modifier,
* shortcut,
* text input,
* navigation key,
* function key.

Phải phân biệt:

Ctrl+C
Ctrl+V
Ctrl+Shift+S
Alt+Tab
Enter
Escape
Delete

với việc người dùng nhập text.

Text nhập liên tục phải được gom thành:

TYPE_TEXT

thay vì lưu hàng loạt event ký tự riêng lẻ.

Ví dụ:

TYPE_TEXT "Summer Nails".

---

# 11. Privacy Engine

Privacy Engine là thành phần bắt buộc nằm trước Storage.

Luồng:

Collector
→ Privacy Filter
→ Storage.

Không lưu dữ liệu nhạy cảm rồi mới redact sau.

Phải nhận diện và loại bỏ:

* password,
* OTP,
* authentication token,
* API secret khi nhận diện được,
* browser password field,
* Windows credential UI,
* dữ liệu thẻ thanh toán,
* trường được cấu hình là sensitive.

Ví dụ:

{
"type": "text_input",
"target": "password",
"value": "[REDACTED]",
"length": 14
}

Phải hỗ trợ:

* application blocklist,
* domain blocklist,
* window-title rule,
* sensitive DOM selector,
* UIA password property,
* regex secret detection,
* admin privacy policy.

---

# 12. Active Application Tracking

Luôn xác định được ứng dụng foreground.

Lưu:

* process,
* PID,
* executable,
* application name,
* HWND,
* window title,
* bounds,
* state,
* monitor.

Track các sự kiện:

* application opened,
* application closed,
* foreground changed,
* window opened,
* window closed,
* window moved,
* window resized,
* minimize,
* maximize,
* restore.

---

# 13. Windows UI Automation

Đây là nguồn semantic chính cho ứng dụng desktop.

Tại vị trí interaction, cố lấy:

* ControlType,
* Name,
* AutomationId,
* ClassName,
* FrameworkId,
* BoundingRectangle,
* Value,
* HelpText,
* IsEnabled,
* IsKeyboardFocusable,
* parent,
* ancestors,
* relevant children.

Ví dụ thay vì:

CLICK x=1212 y=634

phải cố lưu:

Application: Photoshop
ControlType: Button
Name: Export
AutomationId: exportButton.

UI Automation timeout hoặc failure không được làm block input hook.

Nếu không lấy được target thì fallback sang:

* screenshot,
* coordinate,
* active window.

---

# 14. Browser Extension

Phải có companion extension cho:

* Chrome,
* Edge.

Extension gửi event về Recorder Service.

Track:

* URL,
* domain,
* page title,
* tab,
* window,
* navigation,
* new tab,
* close tab,
* back,
* forward,
* reload.

Interaction:

* click,
* double click,
* input,
* change,
* submit,
* scroll,
* selection,
* drag/drop,
* keyboard,
* focus,
* blur.

DOM target cố lấy:

* tag,
* role,
* visible text,
* aria-label,
* name,
* id,
* class,
* href,
* placeholder,
* input type,
* bounding box,
* DOM path,
* CSS selector,
* XPath fallback.

Không dump toàn bộ DOM theo từng frame.

Chỉ lưu snapshot hoặc diff tại state quan trọng.

---

# 15. Screenshot Capture

Screenshot là evidence bắt buộc.

Hỗ trợ multi-monitor.

Tại mỗi action quan trọng:

STATE BEFORE
→ ACTION
→ STATE AFTER.

Capture có thể gồm:

before
action
after.

After state không được chỉ dựa trên delay cố định.

Recorder có thể capture:

+200ms
+500ms
+1000ms

và sử dụng UI/screen stabilization để xác định state đã ổn định.

Ảnh dùng WebP để giảm dung lượng.

---

# 16. Video Capture

Video là secondary evidence.

Hỗ trợ:

OFF
EVENT_BASED
CONTINUOUS.

Continuous mode phải:

* capture đúng monitor,
* đồng bộ cursor,
* đồng bộ timestamp,
* map được event sang video timestamp.

Ví dụ:

Event #827
→ video 13:21.400–13:25.800.

AI hoặc người xem sau này không phải xem toàn bộ video.

---

# 17. Clipboard

Track:

* copy,
* cut,
* paste.

Cố xác định:

* source application,
* source target,
* destination application,
* destination target.

Metadata:

* content_type,
* length,
* hash,
* source,
* destination.

Không mặc định lưu toàn bộ clipboard nhạy cảm.

---

# 18. File Operations

Track file operation liên quan workflow:

* open,
* create,
* save,
* save as,
* rename,
* move,
* copy,
* delete,
* download,
* upload,
* export.

Metadata:

* filename,
* extension,
* directory,
* size,
* created time,
* modified time,
* originating application.

Không copy nội dung tất cả file vào trajectory.

---

# 19. File Dialog

Phải nhận diện các Windows File Dialog:

* Open,
* Save,
* Save As,
* Select Folder,
* Upload,
* Download.

Track:

* selected file,
* selected directory,
* filename,
* extension,
* filter,
* confirm,
* cancel.

---

# 20. Drag & Drop

Drag/drop phải được normalize thành interaction riêng.

Không chỉ ghi mouse down và mouse up.

Event phải cố xác định:

* source,
* destination,
* path,
* duration.

Ví dụ:

Explorer/dog.png
→ drag
→ Chrome/Upload Area.

---

# 21. Scroll

Track:

* direction,
* distance,
* target/container,
* before state,
* after state.

Phải cố phân biệt:

* page scroll,
* modal scroll,
* dropdown scroll,
* spreadsheet scroll,
* panel scroll.

---

# 22. Wait / Idle

Wait là dữ liệu quan trọng.

Phân biệt:

USER_IDLE
SYSTEM_WAIT
APP_LOADING
NETWORK_WAIT
PROCESSING.

Ví dụ:

Click Generate
→ WAIT 13.7s
→ Result appears.

Không tự suy luận rằng mọi khoảng không có input đều giống nhau.

---

# 23. System/Application State Change

Recorder phải thu các state change khi xác định được:

* dialog appeared,
* dialog disappeared,
* menu opened,
* page changed,
* toast appeared,
* error appeared,
* loading started,
* loading ended,
* progress changed,
* file created,
* download completed,
* new window appeared.

Nguồn dữ liệu:

* UIA events,
* DOM MutationObserver,
* Window events,
* Filesystem watcher,
* Screen difference.

---

# 24. Canonical Action

Đơn vị dữ liệu chuẩn của hệ thống phải là:

STATE BEFORE
→ ACTION
→ STATE AFTER.

Ví dụ:

{
"event_id": 842,

"before": {
"app": "chrome",
"url": ".../products",
"screenshot": "842_before.webp"
},

"action": {
"type": "click",
"target": {
"role": "button",
"name": "Save"
}
},

"after": {
"url": ".../products/123",
"ui_changes": [
"toast: Product saved"
],
"screenshot": "842_after.webp"
}
}

Đây là schema trung tâm.

---

# 25. Event Correlation

Các collector không được tự tạo các timeline độc lập.

Nguồn:

Mouse Hook
Keyboard Hook
UIA
Browser Extension
Window Monitor
Filesystem
Screenshot
Clipboard.

Tất cả phải đi qua:

Event Bus
→ State Correlator
→ Canonical Timeline.

Ví dụ một click browser có thể tạo:

mouse_down
mouse_up
DOM click
UIA focus
screen change
URL change.

Correlator phải gom chúng thành một canonical action nếu chúng thuộc cùng interaction.

---

# 26. RAW và NORMALIZED

Phải lưu hai tầng.

## RAW

Dữ liệu gốc từ collector.

Ví dụ:

mouse_down
mouse_up
DOM click
UIA focus
screen changed.

## NORMALIZED

Dữ liệu đã correlate.

Ví dụ:

CLICK "Checkout".

Không được xóa raw data sau normalization.

Sau này thay đổi thuật toán correlation vẫn có thể regenerate normalized dataset.

---

# 27. Timestamp

Tất cả collector phải sử dụng timestamp có thể correlation chính xác.

Cần:

* wall clock timestamp,
* monotonic timestamp,
* timezone,
* millisecond hoặc tốt hơn.

Video, browser, UIA, input và filesystem phải có thể map chung timeline.

---

# 28. Local Spool

Không upload trực tiếp session đang record.

Local storage:

spool/
recording/
pending_upload/
uploading/
uploaded/
failed/

Luồng:

recording
→ pending_upload
→ uploading
→ uploaded.

Session mới tiếp tục capture trong khi session cũ upload.

---

# 29. Hourly Rotation

Đúng giờ:

09:00:00.000

Session trước được đóng logical boundary.

Session mới nhận event tiếp theo ngay lập tức.

Finalization của session cũ chạy background.

Không được:

Stop Recorder
→ Compress
→ Upload
→ Start Recorder.

---

# 30. Compression

Sau khi session đóng:

session
→ finalize
→ Zstd compression
→ encryption
→ chunking
→ upload.

Không block Recorder Service.

---

# 31. Upload

Uploader Service upload session lên server.

Không upload một file lớn duy nhất mà không resume.

Session phải được chia chunk.

Khuyến nghị:

64–256 MB/chunk.

Server track:

chunk index
chunk hash
received status.

Nếu upload fail:

chỉ retry chunk lỗi.

---

# 32. Checksum

Mỗi chunk có checksum.

Toàn session có checksum cuối.

Khuyến nghị:

SHA-256.

Client chỉ được đánh dấu session UPLOADED sau khi server trả:

SESSION_ACCEPTED.

Chỉ khi đó mới được áp dụng local retention/delete policy.

---

# 33. Mất mạng

Nếu mất mạng:

Recorder vẫn hoạt động bình thường.

Session tiếp tục được tạo:

09:00
10:00
11:00
12:00...

Tất cả chuyển sang:

pending_upload.

Khi mạng trở lại:

Uploader upload session cũ nhất trước.

---

# 34. Disk Protection

Phải có cơ chế chống đầy ổ.

Ví dụ policy:

<70%

Normal mode.

70–85%

Giảm video bitrate hoặc capture quality.

85–92%

Tắt continuous video.

Giữ:

event
UIA
DOM
screenshot quan trọng.

> 92%

Chuyển sang critical capture mode.

Ưu tiên dữ liệu:

1. Canonical Events
2. Raw Events
3. UIA / DOM
4. Important Screenshots
5. General Screenshots
6. Video

Không hy sinh event data để giữ video.

---

# 35. Shutdown

Khi Windows shutdown hoặc user logout:

Recorder cố:

* flush pending buffer,
* close current chunk,
* cập nhật session metadata.

Không cần chờ upload xong mới shutdown.

---

# 36. Crash Recovery

Nếu máy mất điện hoặc process crash:

Session không được mất toàn bộ.

Recorder phải dùng:

* append-only write,
* periodic flush,
* chunk files,
* WAL nếu dùng SQLite.

Sau reboot:

phát hiện unfinished session
→ validate
→ recover
→ finalize
→ queue upload.

Session status:

RECOVERED.

---

# 37. Session Storage Format

Một session có cấu trúc:

session_xxx/

manifest.json

events.raw.jsonl

events.normalized.jsonl

session.sqlite

screenshots/

video/

browser/

uia/

files/

annotations/

diagnostics/

Có thể đóng thành:

`.trajectory`

hoặc

`.trajectory.zst`.

---

# 38. Schema Version

Ngay từ phiên bản đầu:

{
"schema": "trajectory",
"schema_version": "1.0"
}

Mọi major schema change phải versioned.

Không thay đổi silent schema sau khi đã tích lũy dataset.

---

# 39. Server

Server phải có API nhận session.

Server lưu:

* employee_id,
* machine_id,
* session_id,
* started_at,
* ended_at,
* event count,
* screenshot count,
* duration,
* archive size,
* checksum,
* upload progress,
* upload completed time,
* processing status.

---

# 40. Server Storage

Khuyến nghị tách:

Metadata
→ PostgreSQL.

Trajectory object
→ object storage hoặc filesystem storage.

Không lưu screenshot/video blob trực tiếp vào PostgreSQL.

Cấu trúc:

machine/
year/
month/
day/
hour/
trajectory.

---

# 41. Server Idempotency

Upload cùng session nhiều lần không được tạo duplicate.

Khóa chính dựa trên:

session_id

* machine_id.

Nếu uploader retry:

server tiếp tục upload đang dang dở.

---

# 42. Encryption

Dữ liệu local spool phải hỗ trợ encryption.

Dữ liệu truyền lên server:

TLS.

Archive có thể encrypt riêng nếu yêu cầu bảo mật nội bộ cao.

Credential uploader phải lưu bằng Windows Credential Manager hoặc cơ chế tương đương.

Không hardcode token trong executable/config plaintext.

---

# 43. Machine Identity

Mỗi máy phải có `machine_id` riêng.

Không dùng hostname làm identity duy nhất.

Machine registration:

Recorder install
→ generate machine_id
→ register server
→ nhận credential.

---

# 44. Employee Mapping

Server có thể map:

machine_id
→ Windows user
→ employee_id.

Không hardcode employee trong recorder binary.

Nếu nhiều user đăng nhập cùng máy phải phân biệt được session.

---

# 45. Diagnostics

Recorder phải tự log diagnostics:

* service start,
* service stop,
* hook status,
* UIA failure,
* browser extension disconnected,
* screenshot failure,
* video failure,
* disk warning,
* upload failure,
* recovery,
* server response.

Diagnostics không được chứa password/secret.

---

# 46. Health Monitoring

Recorder Service phải expose health status cho local UI:

RECORDER_OK
UPLOADER_OK
SERVER_CONNECTED
BROWSER_CONNECTED
DISK_OK.

Server cũng phải biết:

machine online/offline
last heartbeat
last session received.

---

# 47. Performance

Mục tiêu khi không quay video liên tục:

Idle CPU:

<1%.

Normal recording:

<5%.

RAM:

khoảng 150–250 MB hoặc thấp hơn nếu có thể.

Input latency:

không được cảm nhận được.

Input Hook tuyệt đối không làm:

disk I/O
network call
screenshot compression
UIA traversal dài.

Hook chỉ enqueue event.

Các tác vụ nặng chạy worker riêng.

---

# 48. Queue và Backpressure

Tất cả capture source đưa event vào bounded queue.

Có:

* priority,
* backpressure,
* drop strategy.

Không được drop canonical input event trước screenshot/video.

Nếu overload:

drop video frame trước.

Sau đó giảm screenshot.

Không drop click/key/window event nếu còn khả năng lưu.

---

# 49. Browser Extension Disconnect

Nếu browser extension mất kết nối:

Recorder vẫn:

* capture input,
* screenshot,
* active window,
* UIA nếu được.

Event browser đánh dấu:

semantic_browser_data_missing = true.

Khi extension reconnect thì tiếp tục bình thường.

---

# 50. UIA Failure

UI Automation có thể:

* timeout,
* không hỗ trợ app,
* treo trên app đặc biệt.

Không được để UIA block recorder.

Có timeout cứng.

Fallback:

coordinate

* screenshot
* window metadata.

---

# 51. Multi-Monitor

Phải hỗ trợ nhiều màn hình.

Mọi interaction lưu:

monitor_id
screen coordinate
normalized coordinate.

Screenshot phải biết ảnh thuộc monitor nào.

Video nếu bật continuous phải hỗ trợ:

* từng monitor riêng,
  hoặc
* desktop canvas chung.

---

# 52. Display Change

Nếu người dùng:

* cắm thêm màn hình,
* rút màn hình,
* đổi resolution,
* đổi scale,
* đổi orientation,

recorder phải cập nhật display topology.

Session metadata ghi lại thay đổi.

---

# 53. Lock Screen

Khi Windows lock:

không tiếp tục capture desktop protected/lock UI.

Record event:

SESSION_LOCK.

Khi unlock:

SESSION_UNLOCK.

Timeline vẫn liên tục.

---

# 54. Sleep / Hibernate

Track:

SYSTEM_SLEEP
SYSTEM_RESUME.

Không coi khoảng sleep là idle workflow.

---

# 55. Session Viewer

Tauri hoặc web internal tool phải đọc trajectory.

Timeline ví dụ:

03:20 Chrome
Open Shopify

03:22 CLICK Products

03:23 CLICK Add product

03:24 TYPE_TEXT "Custom Pet Nails"

03:25 UPLOAD dog.png

03:28 CLICK Save

03:28 WAIT 1.4s

03:30 Product saved

03:31 Photoshop.

Click event phải xem được:

Before
Action
After.

Kèm:

* screenshot,
* target metadata,
* UIA,
* DOM,
* raw event,
* file event,
* timing.

---

# 56. Search / Filter

Session viewer hỗ trợ filter:

application
action type
domain
file extension
window
upload
download
typing
dialog
error
wait
drag/drop.

Có thể search text target hoặc element name.

---

# 57. Annotation

Hệ thống hỗ trợ annotation sau khi record.

Có thể đánh dấu:

* workflow start,
* workflow end,
* important,
* error,
* accidental action,
* exception.

Annotation có:

* event range,
* text,
* author,
* created_at.

Dữ liệu annotation tách khỏi raw trajectory.

---

# 58. Không Thu Thập Dữ Liệu Vô Nghĩa

“Track mọi thứ” có nghĩa:

Track đủ để tái dựng workflow.

Không có nghĩa:

lưu mọi byte hệ thống.

Không cần:

* mouse position 60 lần/giây,
* full DOM mỗi frame,
* full UIA tree liên tục,
* lossless screen recording,
* nội dung toàn bộ file,
* password/OTP.

Nguyên tắc:

**Lossless về hành động và workflow state.
Selective về dữ liệu phụ và dữ liệu nhạy cảm.**

---

# 59. Thứ Tự Ưu Tiên Dữ Liệu

Nếu có giới hạn CPU/disk/network:

1. Canonical Action
2. Raw Input/Event
3. App/Window metadata
4. Browser DOM / UIA
5. State change
6. Important screenshot
7. General screenshot
8. Video

Video luôn là dữ liệu có thể hy sinh đầu tiên.

---

# 60. Yêu Cầu Server Không Ảnh Hưởng Client

Client phải có khả năng record trong nhiều ngày kể cả server unavailable.

Server không được nằm trên critical capture path.

Không API call nào được thực hiện trực tiếp từ:

* mouse hook,
* keyboard hook,
* screen callback,
* UIA callback.

---

# 61. Definition of Done

Một nhân viên thực hiện workflow 30 phút:

Chrome
→ Excel
→ Explorer
→ Photoshop
→ Chrome.

Sau khi dữ liệu được upload, một kỹ sư chưa từng xem nhân viên làm phải có khả năng dùng trajectory để xác định:

* nhân viên mở ứng dụng nào,
* chuyển ứng dụng khi nào,
* nhìn thấy trạng thái gì,
* click element nào,
* nhập dữ liệu gì,
* dùng shortcut gì,
* copy từ đâu,
* paste sang đâu,
* mở file nào,
* chọn file nào,
* upload file nào,
* download file nào,
* drag object nào,
* thả vào đâu,
* scroll container nào,
* dialog nào xuất hiện,
* người dùng xác nhận/cancel gì,
* app chờ bao lâu,
* kết quả thao tác là gì,
* output cuối nằm ở đâu.

Workflow đi qua boundary giữa hai session theo giờ vẫn phải nối lại được bằng global timeline.

---

# 62. Điều Kiện Nghiệm Thu Production

Phần mềm chỉ được coi là đạt khi đáp ứng đồng thời:

1. Windows boot tự chạy Recorder Service.

2. Nhân viên không phải tự Start Record.

3. Recorder hoạt động liên tục trong thời gian user login.

4. Session tự partition theo từng giờ.

5. Không mất event tại boundary giữa hai session.

6. Session cũ upload nền trong khi session mới tiếp tục record.

7. Mất Internet không làm dừng capture.

8. Server down không làm dừng capture.

9. Restart/mất điện có khả năng recover session.

10. UI crash không làm dừng recorder.

11. Browser extension crash không làm dừng recorder.

12. UIA fail không làm dừng recorder.

13. Disk gần đầy có degradation policy.

14. Password/OTP không được ghi plaintext.

15. Session upload hỗ trợ resume.

16. Server verify checksum trước khi accept.

17. Upload retry không tạo duplicate.

18. Raw data được giữ lại.

19. Normalized data có thể regenerate từ raw.

20. Một workflow đi qua nhiều app có thể tái dựng được từ trajectory.

---

# 63. Kết Luận Kiến Trúc

Kiến trúc cuối cùng:

Windows Boot

↓

Rust Recorder Service

↓

Continuous Event Stream

↓

Input + Window + UIA + Browser + Screen + File + Clipboard

↓

Privacy Engine

↓

Event Bus

↓

State Correlator

↓

Raw Timeline + Canonical Timeline

↓

Session Router

↓

Hourly Session

↓

Local Spool

↓

Background Finalize

↓

Compress + Encrypt + Chunk

↓

Uploader Service

↓

Company Server

↓

Trajectory Storage

↓

Session Viewer / Dataset

↓

AI Skill Pipeline sau này.

Nguyên tắc quan trọng nhất của toàn hệ thống:

**Recorder chạy liên tục.
Session chỉ là cách chia dữ liệu.
Network không nằm trên đường capture.
Raw trajectory không được mất.
Video chỉ là evidence phụ.
Dữ liệu phải đủ để tái dựng workflow của người dùng.**
