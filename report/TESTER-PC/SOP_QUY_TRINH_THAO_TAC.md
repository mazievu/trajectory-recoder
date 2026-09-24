# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `TESTER-PC`  
**Vai trò đảm nhiệm**: **QA / Testing & Infrastructure Verification**  
**Phòng ban / Bộ phận**: Quality Assurance  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:04 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **3,768** thao tác ghi nhận thực tế (3,083 thao tác nghiệp vụ, đã khử 2,601 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Kiểm thử phiên bản client recorder mới, chạy test stress kết nối LAN và kiểm tra tính toàn vẹn của dữ liệu telemetry.
- **Bộ phần mềm tác nghiệp chính**: `chrome.exe`, `powershell.exe`, `explorer.exe`, `cmd.exe`.
- **Định mức chu kỳ hoàn thành**: `20 - 40 phút / test suite kiểm thử`.
- **Tổ hợp phím tắt khuyến nghị**: `Ctrl+Shift+I` (DevTools), `F12` (Inspect elements), `Up/Down Arrow` (History cmd).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

*Giai đoạn này được thực hiện kết hợp trong luồng công việc chính hoặc tự động hóa.*

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `Click` | `chrome.exe` | Bấm [Text] "Phân tích video tham chiếu & Tạo kịch bản dựng (Reference Analysis & Timeline Generation)" trên trình duyệt | `(2799, 549)` |
| **B02** | `TypeText` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `Giao diện UIA` |
| **B03** | `Click` | `chrome.exe` | Bấm [Edit] "Trò chuyện với ChatGPT" trên trình duyệt | `(2220, 984)` |
| **B04** | `TypeText` | `chrome.exe` | Bấm [Edit] "Trò chuyện với ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B05** | `Click` | `chrome.exe` | Bấm [TabItem] "Báo giá phát triển website MARIVO - 15.000.000 VNĐ - Memory usage - 104 MB" trên trình duyệt | `(2193, 0)` |
| **B06** | `Click` | `chrome.exe` | Bấm [TabItem] "SRS — Feature GTF Video Studio - Lark Docs - High memory usage - 878 MB" trên trình duyệt | `(2450, 0)` |
| **B07** | `Click` | `chrome.exe` | Bấm [Button] "Gửi câu lệnh" trên trình duyệt | `(2497, 990)` |
| **B08** | `Click` | `Lark.exe` | Bấm "SRS — Feature GTF Video Studio - Lark Docs - High memory usage - 808 MB" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `(2447, 0)` |
| **B09** | `Click` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `(2673, 426)` |
| **B10** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `Giao diện UIA` |
| **B11** | `Click` | `WINWORD.EXE` | Thao tác Click trên ứng dụng WINWORD.EXE | `(833, 570)` |
| **B12** | `TypeText` | `WINWORD.EXE` | Bấm [Document] "SRS_Feature_GTF_Video_Studio_v1.3_Media_Description_Index.docx  -  Read-Only  -  Compatibility Mode" trên phần mềm WINWORD.EXE | `Giao diện UIA` |
| **B13** | `Click` | `chrome.exe` | Tương tác trên trang web: 'Thiết kế lại hệ thốngburugburu - Google Chrom...' | `(2779, 685)` |
| **B14** | `TypeText` | `chrome.exe` | Tương tác trên trang web: 'Thiết kế lại hệ thốngburugburu - Google Chrom...' | `Giao diện UIA` |
| **B15** | `DragDrop` | `Lark.exe` | Tương tác trên giao diện Lark (Trao đổi nhóm, duyệt brief hoặc gửi file bàn giao) | `Giao diện UIA` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B16** | `DragDrop` | `CapCut.exe` | Bấm nút [Text] "Công nghệ chính: FFmpeg. Có thể sử dụng Playwright + Google Flow CDP (app/media.py) trong các trường hợp cần xử lý hoặc generate media thông qua workflow bên ngoài." trên giao diện CapCut | `Giao diện UIA` |
| **B17** | `Click` | `msedge.exe` | Bấm [Hyperlink] "4. Media Description Index — Phân tích trước video trong Library" trên trình duyệt | `(2005, 302)` |
| **B18** | `Click` | `chrome.exe` | Bấm [Edit] "Mô tả" trên trình duyệt | `(510, 597)` |
| **B19** | `TypeText` | `chrome.exe` | Bấm [Edit] "Mô tả" trên trình duyệt | `Giao diện UIA` |
| **B20** | `Click` | `chrome.exe` | Bấm [Edit] "Optional — vai trò/hướng dẫn cố định cho model" trên trình duyệt | `(3196, 539)` |
| **B21** | `TypeText` | `chrome.exe` | Bấm [Edit] "Optional — vai trò/hướng dẫn cố định cho model" trên trình duyệt | `Giao diện UIA` |
| **B22** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (3218, 844): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(3218, 844)` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B23** | `Click` | `chrome.exe` | Bấm [Button] "Đăng nhập" trên trình duyệt | `(2803, 713)` |
| **B24** | `Click` | `chrome.exe` | Bấm [ListItem] "Duyệt phát hành (Final Review Hand-off): Đóng gói episode_manifest.json và storyboard.html để chuẩn bị đăng tải." trên trình duyệt | `(3184, 955)` |
| **B25** | `Click` | `TextInputHost.exe` | Bấm [DataItem] "Chuẩn bị thư viện video nguồn (Local Media Library): Hệ thống sử dụng một folder local chứa sẵn nhiều video đa dạng đã được creator chuẩn bị trước. Hệ thống quét, lập danh mục và ghi nhận metadata/tim See more Pin item" trên phần mềm TextInputHost.exe | `(3126, 843)` |
| **B26** | `Click` | `chrome.exe` | Bấm [Hyperlink] "07 Huỷ một việc đã giao" trên trình duyệt | `(629, 741)` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **3,768** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **81.8%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **2,601 (69.0%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **0.6%** (20 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **83.1%** (2,562 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **15.4%** (476 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **0.8%** (25 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Ghi nhận chi tiết bước tái hiện lỗi (Steps to Reproduce), log output và chụp ảnh màn hình khi phát hiện bug.
2. Kiểm thử khả năng tự động khôi phục (Auto-reconnect & Queue Buffer) của client khi ngắt kết nối mạng LAN đột ngột.
3. Xác nhận tính chính xác của dữ liệu ghi nhận (Window title, Target UI element) trước khi duyệt release bản client mới.
