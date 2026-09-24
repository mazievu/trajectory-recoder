# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `EDIT-NGOCMINH`  
**Vai trò đảm nhiệm**: **Tech Lead / AI Developer Workstation**  
**Phòng ban / Bộ phận**: Engineering & Technology  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:04 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **46,740** thao tác ghi nhận thực tế (36,894 thao tác nghiệp vụ, đã khử 32,757 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Phát triển công cụ tự động hóa, kiểm thử mô hình AI, quản trị hạ tầng phần mềm nội bộ và hỗ trợ kỹ thuật dữ liệu.
- **Bộ phần mềm tác nghiệp chính**: `chrome.exe`, `Code.exe`, `cmd.exe`, `Lark.exe`, `explorer.exe`.
- **Định mức chu kỳ hoàn thành**: `30 - 60 phút / module tính năng hoặc script`.
- **Tổ hợp phím tắt khuyến nghị**: `Ctrl+` ` (Terminal VS Code), `Ctrl+P` (Mở file nhanh), `Ctrl+K Ctrl+S` (Phím tắt IDE).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `Click` | `explorer.exe` | Chọn tệp tin / asset trong Windows Explorer | `(1246, 500)` |
| **B02** | `TypeText` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B03** | `Click` | `chrome.exe` | Bấm [Hyperlink] "Pull requests  (2)" trên trình duyệt | `(2152, 138)` |
| **B04** | `TypeText` | `Lark.exe` | Bấm "Load older messages, showing 123 of 139 Conversation Log User message Agent response 2:19 PM Copy Good response Bad response User message Agent response 2:24 PM Copy Good response Bad response User message Agent response 2:31 PM Copy Good response Bad response Scroll to Bottom Ask anything, @ to mention, / for actions Add context Select model, current: Gemini 3.8 Flash High Record voice memo Send message" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `Giao diện UIA` |

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B05** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `Giao diện UIA` |
| **B06** | `Click` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `(1408, 210)` |
| **B07** | `Click` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `(558, 973)` |
| **B08** | `TypeText` | `msedge.exe` | Bấm [Group] "Đúng, bạn đang hiểu đúng hướng hơn rồi. Trong task này, có thể coi E-commerce là phạm vi lớn, còn bạn tập trung nghiên cứu 2 mô hình bên trong là POD (Print on Demand) và Dropshipping. Mục tiêu của DeerFlow không phải chỉ thu thập thông tin chung về hai ngành này, mà phải tìm ra những topic cụ thể đang có nhu cầu cao. Bạn cần tìm cho mỗi topic các tín hiệu như: Mức độ quan tâm: Google Trends, lượng tìm kiếm, Reddit/YouTube/TikTok discussions. Dấu hiệu có người trả tiền: ebook/sách có BSR tốt, nhiều review, khóa học hoặc sản phẩm liên quan phổ biến. Xu hướng: đang tăng, ổn định hay giảm. Pain point: người dùng đang gặp vấn đề gì và muốn học gì. Competition: chủ đề có nhiều content/sách cạnh tranh hay chưa. Ví dụ với POD, DeerFlow có thể phát hiện các topic như Etsy POD, AI design for POD, POD niche research, Etsy SEO, Printify vs Printful... Sau đó bạn so sánh xem topic nào có tín hiệu demand mạnh nhất. Với Dropshipping, có thể là winning product research, TikTok organic dropshipping, Meta Ads, supplier sourcing, AI automation, Shopify dropshipping... Có một điểm cần chỉnh trong câu “lượng mua cao”: thường bạn không lấy được chính xác số người mua ebook. Vì vậy task thực tế nên hiểu là: Ví dụ nếu DeerFlow thấy:" trên trình duyệt | `Giao diện UIA` |
| **B09** | `TypeText` | `msedge.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B10** | `DragDrop` | `chrome.exe` | Bấm [Button] "All Bookmarks" trên trình duyệt | `Giao diện UIA` |
| **B11** | `Click` | `System` | Bấm [Edit] "Name" trên phần mềm System | `(693, 711)` |
| **B12** | `DragDrop` | `chrome.exe` | Bấm [Button] "Close" trên trình duyệt | `Giao diện UIA` |
| **B13** | `Click` | `chrome.exe` | Bấm [Button] "Reload" trên trình duyệt | `(95, 71)` |
| **B14** | `Click` | `chrome.exe` | Bấm [Button] "Amazon 26" trên trình duyệt | `(426, 321)` |
| **B15** | `Click` | `chrome.exe` | Bấm [Button] "Lọc & xếp hạng theo chỉ số" trên trình duyệt | `(388, 268)` |
| **B16** | `Click` | `chrome.exe` | Bấm [Image] "TEOYALL 800Pcs Lint Free Nail Wipes, Non-Woven Nail Lash Glue Cleaning Pad | Nail art Prep. Cleans nail polish residueWipes lash glue from tools. Soft absorbent. 800 pcs for salon and home" trên trình duyệt | `(935, 457)` |
| **B17** | `Click` | `chrome.exe` | Bấm [Button] "Etsy 88939" trên trình duyệt | `(662, 310)` |
| **B18** | `Click` | `chrome.exe` | Bấm [Image] "Handmade Y2K Press On Nails, Pink Polka Dot Floral Square, Funky Vintage Nail Art" trên trình duyệt | `(469, 482)` |
| **B19** | `TypeText` | `Notepad.exe` | Bấm [Document] "Text editor" trên phần mềm Notepad.exe | `Giao diện UIA` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B20** | `Click` | `chrome.exe` | Bấm [Text] "nếu vậy thì tôi sẽ bảo nó Tìm các topic trong POD và Dropshipping có mức độ quan tâm cao và có bằng chứng cho thấy người dùng sẵn sàng chi tiền để học hoặc giải quyết vấn đề đó  " trên trình duyệt | `(534, 858)` |
| **B21** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (790, 892): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(790, 892)` |
| **B22** | `Click` | `chrome.exe` | Bấm [TabItem] "Làm video AI nông dân - Memory usage - 206 MB" trên trình duyệt | `(652, 46)` |
| **B23** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1829, 154): Bảng Thuộc Tính (Inspector) [Chỉnh Video / Âm thanh / Tốc độ / Hoạt ảnh / Màu sắc] | `(1829, 154)` |
| **B24** | `Click` | `chrome.exe` | Bấm [Button] "Chuyển nền sáng/tối" trên trình duyệt | `(1854, 174)` |
| **B25** | `Click` | `chrome.exe` | Bấm [Edit] "Tên dự án *" trên trình duyệt | `(133, 263)` |
| **B26** | `TypeText` | `chrome.exe` | Bấm [Edit] "Tên dự án *" trên trình duyệt | `Giao diện UIA` |
| **B27** | `Click` | `chrome.exe` | Bấm [Text] "Phân tích AI" trên trình duyệt | `(448, 460)` |
| **B28** | `DragDrop` | `chrome.exe` | Bấm [Document] "Cơ chế đẩy xu hướng của youtube - Tìm trên Google" trên trình duyệt | `Giao diện UIA` |
| **B29** | `DragDrop` | `chrome.exe` | Bấm [Edit] "Hỏi bất cứ điều gì" trên trình duyệt | `Giao diện UIA` |
| **B30** | `Click` | `chrome.exe` | Bấm [Button] "✨ Task Đơn Lẻ" trên trình duyệt | `(110, 261)` |
| **B31** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1303, 678): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(1303, 678)` |
| **B32** | `Click` | `chrome.exe` | Bấm [Text] "🔐 THÔNG TIN TÀI KHOẢN & MÃ 2FA XÁC THỰC" trên trình duyệt | `(431, 309)` |
| **B33** | `Click` | `chrome.exe` | Bấm [Text] "Trọng Tình" trên trình duyệt | `(2448, 359)` |
| **B34** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (2482, 691): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(2482, 691)` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B35** | `DragDrop` | `chrome.exe` | Bấm [Text] "interests: thì cần phải tìm kiếm cả dữ liệu lịch sử để làm tính biến động của topic(sản phẩm đó) như v thì khi làm bằng chứng ta có thể biểu diễn dưới dạng đồ thị để đánh giá xem topic đó có thật sự có nhu cầu cao, và phù hợp với xu thế hiện tại hay ko ?" trên trình duyệt | `Giao diện UIA` |
| **B36** | `DragDrop` | `chrome.exe` | Bấm [Text] "Tức là bây giờ chúng ta sẽ cho deerflow đi tìm hiểu về E-com trên các nền tảng lớn  cụ thể ở đây là POD, Dropshippings. trong đó sẽ đi tìm hiểu về các chỉ số như tôi với b đã thảo luận. từ dữ liệu thu thập được chúng ta sẽ làm thành một ebook có chia mục và tab riêng cho từng chủ đề, mỗi chủ đề lại cần các chỉ số riêng" trên trình duyệt | `Giao diện UIA` |
| **B37** | `TypeText` | `msedge.exe` | Bấm [Document] "CÓ ĐÔI ĐIỀU X HARU HARU - COLDZ REMIX | NHẠC REMIX ĐANG DẦN DẦN HOT 2025 - YouTube" trên trình duyệt | `Giao diện UIA` |
| **B38** | `DragDrop` | `chrome.exe` | Bấm [Text] " Toàn bộ video thành phẩm trong đợt sản xuất" trên trình duyệt | `Giao diện UIA` |
| **B39** | `Click` | `chrome.exe` | Bấm [Button] "Hiện thêm Thông tin tổng quan do AI tạo" trên trình duyệt | `(601, 616)` |
| **B40** | `Click` | `chrome.exe` | Bấm [Button] "Sao chép Tôi đang định làm video về thế giới động vật thì nên làm video như nào, đăng video như nào, làm thế nào để tăng tương tác ban đầu" trên trình duyệt | `(476, 612)` |
| **B41** | `DragDrop` | `chrome.exe` | Bấm [Text] " chuyên dùng cho mục đích này để bạn tìm kiếm cho nhanh." trên trình duyệt | `Giao diện UIA` |
| **B42** | `DragDrop` | `chrome.exe` | Bấm [ListItem] "Bổ sung cảnh báo chặn sớm (Pre-flight Alert): Nếu video đối thủ và kho lệch chủ đề hoàn toàn, hệ thống hiện thông báo đỏ và dừng ngay, không cho bấm sinh kịch bản vô ích." trên trình duyệt | `Giao diện UIA` |
| **B43** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1182, 751): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(1182, 751)` |
| **B44** | `Click` | `chrome.exe` | Bấm [Text] "Máy a rất mạnh nên việc a cho tạo 10 video cùng lúc là đã ít rồi đấy. A bảo là a muốn mỗi một kịch bản đc sinh ra phải đc gen video" trên trình duyệt | `(1272, 878)` |
| **B45** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'Lark...' | `Giao diện UIA` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **46,740** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **78.9%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **32,757 (70.1%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **2.1%** (764 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **96.3%** (35,511 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **1.4%** (520 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **0.3%** (99 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Tuân thủ triệt để nguyên tắc Clean Code: Không commit code chưa qua kiểm thử cục bộ (Local Unit Test).
2. Mọi biến môi trường và khóa API phải lưu trong `.env`, tuyệt đối không hardcode credentials trong source code.
3. Theo dõi log vận hành Docker container và tài nguyên GPU/VRAM định kỳ tránh tràn bộ nhớ.
