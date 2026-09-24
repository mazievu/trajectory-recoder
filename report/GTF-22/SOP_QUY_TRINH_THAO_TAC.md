# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `GTF-22`  
**Vai trò đảm nhiệm**: **Marketing & Ads Operations Specialist**  
**Phòng ban / Bộ phận**: Growth & Ads Operations  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:03 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **18,666** thao tác ghi nhận thực tế (15,719 thao tác nghiệp vụ, đã khử 11,248 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Vận hành chiến dịch quảng cáo đa kênh, đối soát chi phí quảng cáo theo ngày, kiểm tra pixel chuyển đổi và làm báo cáo hiệu suất.
- **Bộ phần mềm tác nghiệp chính**: `chrome.exe`, `Lark.exe`, `explorer.exe`.
- **Định mức chu kỳ hoàn thành**: `30 - 45 phút / phiên đối soát báo cáo ngày`.
- **Tổ hợp phím tắt khuyến nghị**: `Ctrl+Shift+T` (Khôi phục tab ads), `Ctrl+P` (Xuất PDF báo cáo).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `TypeText` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B02** | `Click` | `chrome.exe` | Bấm [Button] "Cài đặt" trên trình duyệt | `(2672, 618)` |

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B03** | `Click` | `chrome.exe` | Bấm [Button] "Phát" trên trình duyệt | `(2124, 307)` |
| **B04** | `Click` | `explorer.exe` | Chọn tệp tin / asset trong Windows Explorer | `(2119, 285)` |
| **B05** | `Click` | `chrome.exe` | Bấm [Button] "Tạm dừng" trên trình duyệt | `(2128, 317)` |
| **B06** | `Click` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `(1726, 447)` |
| **B07** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `Giao diện UIA` |
| **B08** | `Click` | `chrome.exe` | Bấm [Document] "Thẻ mới" trên trình duyệt | `(2138, 400)` |
| **B09** | `Click` | `PID_1888` | Thao tác Click trên ứng dụng PID_1888 | `(1109, 954)` |
| **B10** | `Click` | `browser.exe` | Bấm [Button] "Cốc Cốc - 2 running windows pinned" trên phần mềm browser.exe | `(1093, 1071)` |
| **B11** | `Click` | `chrome.exe` | Tương tác trên trang web: 'Hình trong hình...' | `(2216, 149)` |
| **B12** | `Click` | `chrome.exe` | Bấm [Document] "Omnibox Popup" trên trình duyệt | `(2065, 113)` |
| **B13** | `Click` | `chrome.exe` | Bấm [Hyperlink] "Trang chủ YouTube" trên trình duyệt | `(2065, 133)` |
| **B14** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (2449, 310): Bảng Thuộc Tính (Inspector) [Chỉnh Video / Âm thanh / Tốc độ / Hoạt ảnh / Màu sắc] | `(2449, 310)` |
| **B15** | `DragDrop` | `explorer.exe` | Kéo thả tệp media từ Windows Explorer vào phần mềm tác nghiệp | `Giao diện UIA` |
| **B16** | `Click` | `Lark.exe` | Bấm "Tua đi 10 giây" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `(2142, 225)` |
| **B17** | `Click` | `Lark.exe` | Tương tác trên giao diện Lark (Trao đổi nhóm, duyệt brief hoặc gửi file bàn giao) | `(1699, 450)` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B18** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'Hình trong hình...' | `Giao diện UIA` |
| **B19** | `Click` | `chrome.exe` | Bấm [Button] "Hôm nay: 16 Tháng 9, 2026" trên trình duyệt | `(3646, 333)` |
| **B20** | `TypeText` | `browser.exe` | Bấm [Window] "Hình trong hình" trên phần mềm browser.exe | `Giao diện UIA` |
| **B21** | `Click` | `Lark.exe` | Bấm "Bao giờ gửi a được?" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `(2348, 183)` |
| **B22** | `DragDrop` | `CapCut.exe` | Bấm nút [Text] " các content ads khác đâu e? Gửi update cho a" trên giao diện CapCut | `Giao diện UIA` |
| **B23** | `Click` | `chrome.exe` | Bấm [Button] "Tua đi 10 giây" trên trình duyệt | `(2170, 925)` |
| **B24** | `DragDrop` | `chrome.exe` | Bấm [Text] "Trên content text ads có thêm keyword về style móng để con bot nó phân phối chuẩn hơn. Ví dụ những cô gái cá tính k thể bỏ qua chiếc móng halloween coffin này… " trên trình duyệt | `Giao diện UIA` |
| **B25** | `DragDrop` | `Lark.exe` | Bấm "Trên content text ads có thêm keyword về style móng để con bot nó phân phối chuẩn hơn. Ví dụ những cô gái cá tính k thể bỏ qua chiếc móng halloween coffin này… " trên ứng dụng Lark để gửi thông báo/nộp kết quả | `Giao diện UIA` |
| **B26** | `Click` | `chrome.exe` | Bấm [Button] "Phụ đề phím tắt c" trên trình duyệt | `(2625, 626)` |
| **B27** | `Click` | `chrome.exe` | Bấm [Button] "Nhân bản" trên trình duyệt | `(166, 400)` |
| **B28** | `Click` | `chrome.exe` | Bấm [MenuItem] "Xóa" trên trình duyệt | `(473, 587)` |
| **B29** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (831, 383): Màn Hình Preview [Khung xem trước video & Căn chỉnh bounding box khung hình] | `(831, 383)` |
| **B30** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (817, 470): Màn Hình Preview [Khung xem trước video & Căn chỉnh bounding box khung hình] | `(817, 470)` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B31** | `Click` | `chrome.exe` | Bấm [Text] "Nén ảnh online miễn phí - WebP/AVIF chuẩn SEO, không tải lên máy chủ" trên trình duyệt | `(1721, 344)` |
| **B32** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'Lark...' | `Giao diện UIA` |
| **B33** | `DragDrop` | `chrome.exe` | Bấm [Text] "Các nàng mê móng ngắn nhất định không thể bỏ qua bộ móng dáng coffin chủ đề Halloween này đâu nhé 👻🖤

Vừa dễ thương, cá tính lại còn được cá nhân hóa với tên riêng của bạn—giúp bạn thể hiện phong cách mà chẳng cần để móng dài." trên trình duyệt | `Giao diện UIA` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **18,666** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **84.2%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **11,248 (60.3%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **5.4%** (846 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **89.6%** (14,088 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **4.2%** (654 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **0.8%** (131 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Đối soát số liệu chi tiêu quảng cáo thực tế trên BM/Ads Manager khớp với bảng kế toán Lark Base vào 9h00 sáng.
2. Giám sát cảnh báo tài khoản quảng cáo bị vô hiệu hóa hoặc thẻ thanh toán từ chối để can thiệp trong vòng 15 phút.
3. Cập nhật danh sách từ khóa cấm/hạn chế của nền tảng để nhắc nhở team content điều chỉnh kịp thời.
