# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `DESKTOP-1Q714D7`  
**Vai trò đảm nhiệm**: **Graphic & Vector Designer**  
**Phòng ban / Bộ phận**: Creative & Visual Design  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:02 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **27,002** thao tác ghi nhận thực tế (16,353 thao tác nghiệp vụ, đã khử 2,953 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Thiết kế vector, ấn phẩm banner, thumbnail video, xử lý hình ảnh sản phẩm và đóng gói asset đồ họa.
- **Bộ phần mềm tác nghiệp chính**: `Illustrator.exe`, `Photoshop.exe`, `chrome.exe`, `explorer.exe`.
- **Định mức chu kỳ hoàn thành**: `30 - 60 phút / bộ ấn phẩm banner & thumbnail`.
- **Tổ hợp phím tắt khuyến nghị**: `V` (Selection Tool), `P` (Pen Tool), `Ctrl+Shift+S` (Save As), `Ctrl+Alt+Shift+S` (Export for Web).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `DragDrop` | `System` | Bấm [Pane] "OS_ViewContainer" trên phần mềm System | `Giao diện UIA` |
| **B02** | `DragDrop` | `System` | Thao tác DragDrop trên ứng dụng System | `Giao diện UIA` |
| **B03** | `TypeText` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B04** | `Click` | `System` | Bấm [Button] "Bui - Chrome - 1 cửa sổ đang chạy" trên phần mềm System | `(1149, 1065)` |
| **B05** | `RightClick` | `chrome.exe` | Bấm [Image] "Generated image: Elegant Pink-and-Gold Custom Nail Set" trên trình duyệt | `(893, 472)` |
| **B06** | `TypeText` | `chrome.exe` | Bấm [Edit] "Nội dung tài liệu" trên trình duyệt | `Giao diện UIA` |

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B07** | `Click` | `taskhostw.exe` | Thao tác Click trên ứng dụng taskhostw.exe | `(1502, 699)` |
| **B08** | `TypeText` | `Illustrator.exe` | Gõ nội dung Typography / Thông số thuộc tính trên Illustrator.exe | `Giao diện UIA` |
| **B09** | `Click` | `System` | Thao tác Click trên ứng dụng System | `(1135, 975)` |
| **B10** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'MSCTFIME UI...' | `Giao diện UIA` |
| **B11** | `TypeText` | `PID_24460` | Thao tác TypeText trên ứng dụng PID_24460 | `Giao diện UIA` |
| **B12** | `DragDrop` | `PID_24460` | Thao tác DragDrop trên ứng dụng PID_24460 | `Giao diện UIA` |
| **B13** | `DragDrop` | `PID_24460` | Bấm [Pane] "OS_ViewContainer" trên phần mềm PID_24460 | `Giao diện UIA` |
| **B14** | `DragDrop` | `Illustrator.exe` | Thao tác trên vùng [OS_ViewContainer] (Illustrator.exe) | `Giao diện UIA` |
| **B15** | `TypeText` | `System` | Bấm [Pane] "OS_ViewContainer" trên phần mềm System | `Giao diện UIA` |
| **B16** | `TypeText` | `System` | Thao tác TypeText trên ứng dụng System | `Giao diện UIA` |
| **B17** | `Click` | `Illustrator.exe` | Tương tác công cụ tại (641, 610): Vùng Canvas Thiết Kế [Không gian vẽ & Căn chỉnh vector đồ họa] | `(641, 610)` |
| **B18** | `Click` | `explorer.exe` | Chọn tệp tin / asset trong Windows Explorer | `(1151, 953)` |
| **B19** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'Elva_Product_Description_Standalone.html...' | `Giao diện UIA` |
| **B20** | `DoubleClick` | `Illustrator.exe` | Tương tác công cụ tại (981, 631): Vùng Canvas Thiết Kế [Không gian vẽ & Căn chỉnh vector đồ họa] | `(981, 631)` |
| **B21** | `DoubleClick` | `explorer.exe` | Nhấp đúp mở thư mục chứa tài nguyên media / project trong Windows Explorer | `(969, 633)` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B22** | `DragDrop` | `msedge.exe` | Tương tác trên trang web: 'Elva Nails — Personal Edit · Product Descript...' | `Giao diện UIA` |
| **B23** | `DragDrop` | `msedge.exe` | Bấm [Text] "Prep, then apply" trên trình duyệt | `Giao diện UIA` |
| **B24** | `Click` | `Illustrator.exe` | Tương tác công cụ tại (1170, 1047): Vùng Canvas Thiết Kế [Không gian vẽ & Căn chỉnh vector đồ họa] | `(1170, 1047)` |
| **B25** | `DragDrop` | `Illustrator.exe` | Thao tác trên vùng [đã ghim Microsoft Edge - 1 cửa sổ đang chạy] (Illustrator.exe) | `Giao diện UIA` |
| **B26** | `DragDrop` | `msedge.exe` | Bấm [Text] "Whatever the plan, make it personal." trên trình duyệt | `Giao diện UIA` |
| **B27** | `Click` | `Illustrator.exe` | Tương tác công cụ tại (1164, 1065): Vùng Canvas Thiết Kế [Không gian vẽ & Căn chỉnh vector đồ họa] | `(1164, 1065)` |
| **B28** | `DragDrop` | `Illustrator.exe` | Thao tác trên vùng [Free Transform Tool (E)] (Illustrator.exe) | `Giao diện UIA` |
| **B29** | `Click` | `chrome.exe` | Bấm [Button] "Hiện thêm" trên trình duyệt | `(1636, 468)` |
| **B30** | `Click` | `chrome.exe` | Bấm [Hyperlink] "1 giờ, 15 phút, 20 giây" trên trình duyệt | `(1599, 267)` |
| **B31** | `TypeText` | `browser.exe` | Bấm [Window] "Hình trong hình" trên phần mềm browser.exe | `Giao diện UIA` |
| **B32** | `Click` | `Illustrator.exe` | Tương tác công cụ tại (1747, 1043): Bảng Điều Khiển (Panels - Quản lý Layer, Màu Color, Thuộc tính Properties) | `(1747, 1043)` |
| **B33** | `Click` | `dllhost.exe` | Bấm [Edit] "Tên" trên phần mềm dllhost.exe | `(247, 238)` |
| **B34** | `DoubleClick` | `svchost.exe` | Bấm [Edit] "Tên" trên phần mềm svchost.exe | `(247, 238)` |
| **B35** | `Click` | `Illustrator.exe` | Tương tác công cụ tại (1261, 619): Vùng Canvas Thiết Kế [Không gian vẽ & Căn chỉnh vector đồ họa] | `(1261, 619)` |
| **B36** | `TypeText` | `chrome.exe` | Bấm [ComboBox] "Văn bản nguồn" trên trình duyệt | `Giao diện UIA` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B37** | `Click` | `System` | Bấm [Button] "Chỉ báo Đưa vào Khay Tiếng Anh (Mỹ)
US

Để chuyển đổi phương thức nhập, nhấn
phím Windows + phím cách." trên phần mềm System | `(1760, 1069)` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **27,002** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **60.6%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **2,953 (10.9%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **5.5%** (902 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **83.7%** (13,681 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **10.5%** (1,718 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **0.3%** (52 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Hệ màu thiết kế: Luôn dùng RGB cho ấn phẩm digital/ads, CMYK cho ấn phẩm in ấn bao bì.
2. Tổ chức layer khoa học theo nhóm: `[BG] Background`, `[PROD] Product`, `[TXT] Typography`, `[FX] Effects`.
3. Xuất định dạng PNG-24 cho asset tách nền và JPG nén tối ưu dưới 300KB cho thumbnail web/ads.
