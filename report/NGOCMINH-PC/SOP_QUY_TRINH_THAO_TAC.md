# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `NGOCMINH-PC`  
**Vai trò đảm nhiệm**: **Management / Server & Systems Operations**  
**Phòng ban / Bộ phận**: Operations Management  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:04 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **16,415** thao tác ghi nhận thực tế (13,105 thao tác nghiệp vụ, đã khử 10,924 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Giám sát toàn bộ hạ tầng máy chủ Trajectory Recorder, phê duyệt tài liệu nội bộ, điều phối luồng công việc liên phòng ban.
- **Bộ phần mềm tác nghiệp chính**: `chrome.exe`, `Docker Desktop.exe`, `Lark.exe`, `powershell.exe`, `explorer.exe`.
- **Định mức chu kỳ hoàn thành**: `15 - 30 phút / chu kỳ duyệt vận hành`.
- **Tổ hợp phím tắt khuyến nghị**: `Win+X` (Admin Menu), `Alt+Tab` (Chuyển cửa sổ quản trị).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `TypeText` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `Giao diện UIA` |
| **B02** | `DragDrop` | `explorer.exe` | Kéo thả tệp media từ Windows Explorer vào phần mềm tác nghiệp | `Giao diện UIA` |
| **B03** | `Click` | `chrome.exe` | Bấm [Group] "Your main branch isn't protected main branch 11 Branches 0 Tags Go to file Add file Add file Code Latest commit tinhqk9-netizen commits by tinhqk9-netizen Merge pull request #15 from mazievu/feature/card-metric-form-media-ca… Open commit details success Commit 17a21c2  ·  2 days ago History 65 Commits Folders and files Folders and files" trên trình duyệt | `(276, 222)` |
| **B04** | `Click` | `chrome.exe` | Bấm [Hyperlink] "Repositories  (15)" trên trình duyệt | `(373, 194)` |
| **B05** | `Click` | `chrome.exe` | Bấm [Group] "master branch 8 Branches 17 Tags Go to file Add file Add file Code Latest commit samugit83 claude commits by samugit83 and commits by claude docs: repoint two stale docker-compose.yml line citations Open commit details Commit e2543aa  ·  2 days ago History 1,159 Commits Folders and files Folders and files Repository files navigation Repository files" trên trình duyệt | `(-491, 521)` |
| **B06** | `TypeText` | `chrome.exe` | Bấm [Group] "Load older messages, showing 40 of 79 Conversation Log User message Agent response 3:32 PM Copy Good response Bad response User message Agent response 3:34 PM Copy Good response Bad response luồn 1 vẫn đang chạy ok rồi . giờ chỉ cần coppy sang luồng 2 Add context Select model, current: Gemini 3.8 Flash High Record voice memo Send message" trên trình duyệt | `Giao diện UIA` |
| **B07** | `Click` | `chrome.exe` | Bấm [Group] "Load older messages, showing 75 of 4271 Conversation Log Agent response 10:01 AM Copy Good response Bad response User message Agent response 10:04 AM Copy Good response Bad response User message Agent response 10:07 AM Copy Good response Bad response Ask anything, @ to mention, / for actions Add context Select model, current: Gemini 3.8 Flash High Record voice memo Send message" trên trình duyệt | `(-1317, 1008)` |

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B08** | `Click` | `explorer.exe` | Chọn tệp tin / asset trong Windows Explorer | `(-872, 1054)` |
| **B09** | `Click` | `Lark.exe` | Bấm "Bùi Đăng Minh" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `(410, 663)` |
| **B10** | `Click` | `chrome.exe` | Bấm [Edit] "Chat with ChatGPT" trên trình duyệt | `(-1284, 943)` |
| **B11** | `Click` | `chrome.exe` | Bấm [Button] "Show more" trên trình duyệt | `(-1363, 775)` |
| **B12** | `Click` | `System` | Bấm [Button] "Search" trên phần mềm System | `(775, 1063)` |
| **B13** | `DragDrop` | `chrome.exe` | Bấm [Button] "Chrome" trên trình duyệt | `Giao diện UIA` |
| **B14** | `Click` | `chrome.exe` | Bấm [Hyperlink] "mazievu/crawler-POD" trên trình duyệt | `(382, 482)` |
| **B15** | `Click` | `chrome.exe` | Tương tác trên trang web: 'Chrome Legacy Window...' | `(844, 23)` |
| **B16** | `Click` | `chrome.exe` | Bấm [Button] "Open tool call list Called tool" trên trình duyệt | `(-1482, 541)` |
| **B17** | `RightClick` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `(608, 892)` |
| **B18** | `Click` | `chrome.exe` | Bấm [Button] "companion_ui" trên trình duyệt | `(420, 698)` |
| **B19** | `Click` | `chrome.exe` | Bấm [Hyperlink] "mazievu" trên trình duyệt | `(340, 163)` |
| **B20** | `Click` | `chrome.exe` | Bấm [Hyperlink] "companion_ui" trên trình duyệt | `(888, 438)` |
| **B21** | `Click` | `chrome.exe` | Tương tác trên trang web: 'OLEChannelWnd...' | `(-1264, 885)` |
| **B22** | `Click` | `chrome.exe` | Tương tác trên trang web: 'OleMainThreadWndName...' | `(-1243, 848)` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B23** | `DragDrop` | `chrome.exe` | Bấm [Hyperlink] "Truy xuất và so sánh prompt" trên trình duyệt | `Giao diện UIA` |
| **B24** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (-1270, 915): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(-1270, 915)` |
| **B25** | `DoubleClick` | `chrome.exe` | Bấm [Text] "điểm khác biệt đây là tài liệu được lưu bởi AI . tôi đang nói đến nạp bằng tay " trên trình duyệt | `(-1257, 910)` |
| **B26** | `DragDrop` | `chrome.exe` | Bấm [Group] "Tôi đọc hết bản BA rồi. Tổng thể luồng nghiệp vụ chính đã đúng với ý bạn: Upload thủ công → MarkItDown → Jev phân loại + tag → LLM phân tích ngắn → Library, không có approval queue. BA cũng đã giữ đúng nguyên tắc “upload & quên đi” và không bắt người dùng nhập metadata thủ công. Pasted markdown Nhưng mình chưa chốt plan theo bản này ngay, vì có vài chỗ BA tự đưa ra quyết định thay cho bạn. 1. Lệch lớn nhất: quyền của Leader Bạn nói trước đó: Nhưng BA tự quay về policy cũ của Companion: Leader chỉ quản lý department. Founder chỉ đọc toàn công ty. Chỉ Admin sửa/xóa global. Pasted markdown Đây không phải chi tiết kỹ thuật. Đây là thay đổi requirement của bạn. Cần bạn chốt một câu: Knowledge Library có phải là ngoại lệ, Leader trở lên quản lý toàn bộ tài liệu công ty không? Nếu có, mình sẽ không áp permission mặc định của Companion cho module này. Ta viết permission riêng: Copy staff → manage own documents leader → read/edit/tag/delete all documents founder → read/edit/tag/delete all documents admin → full control + hard delete/system settings Còn nếu muốn đúng RBAC cũ thì giữ như BA. 2. BA tự thêm Personal / Department / Company BA đề xuất user khi upload chọn: Copy" trên trình duyệt | `Giao diện UIA` |
| **B27** | `Click` | `chrome.exe` | Bấm [Text] "có 1 GPT-6 astra đang làm điều này ở trong D:\tools GTF\companion_ui . bạn nên kiểm tra trước khi thực hiện " trên trình duyệt | `(-620, 974)` |
| **B28** | `Click` | `chrome.exe` | Bấm [Button] "Send now: tôi chưa bảo bạn sửa gì đâu nhé" trên trình duyệt | `(-946, 917)` |
| **B29** | `Click` | `chrome.exe` | Bấm [Text] "| Nhóm nội dung | Tên sách | Tác giả |
|---|---|---|
| Quảng cáo và tăng trưởng thương hiệu | How Brands Grow | Byron Sharp |
| Quảng cáo và tăng trưởng thương hiệu | Ogilvy on Advertising | David Ogilvy |
| Quảng cáo và tăng trưởng thương hiệu | Tested Advertising Methods | John Caples, Fred Hahn |
| Quảng cáo và tăng trưởng thương hiệu | Breakthrough Advertising | Eugene M. Schwartz |
| Quảng cáo và tăng trưởng thương hiệu | How Brands Grow Part 2 | Jenni Romaniuk, Byron Sharp |
| Quảng cáo và tăng trưởng thương hiệu | The Long and the Short of It | Les Binet, Peter Field |
| Nghiên cứu khách hàng | Interviewing Users | Steve Portigal |
| Nghiên cứu khách hàng | Just Enough Research | Erika Hall |
| Nghiên cứu khách hàng | Observing the User Experience | Elizabeth Goodman, Mike Kuniavsky, Andrea Moed |
| Nghiên cứu khách hàng | The Mom Test | Rob Fitzpatrick |
| Nghiên cứu khách hàng và phát triển sản phẩm | Continuous Discovery Habits | Teresa Torres |
| Nghiên cứu khách hàng và phát triển sản phẩm | Competing Against Luck | Clayton M. Christensen, Taddy Hall, Karen Dillon, David S. Duncan |
| Phát triển sản phẩm | Escaping the Build Trap | Melissa Perri |
| Phát triển sản phẩm | Testing Business Ideas | David J. Bland, Alexander Osterwalder |
| Chiến lược và cạnh tranh | Good Strategy/Bad Strategy | Richard Rumelt |
| Chiến lược và cạnh tranh | Competitive Strategy | Michael E. Porter |
| Chiến lược và cạnh tranh | Analysis Without Paralysis | Babette E. Bensoussan, Craig S. Fleisher |
| Chiến lược và cạnh tranh | Playing to Win | A.G. Lafley, Roger L. Martin |
| Chiến lược và cạnh tranh | 7 Powers | Hamilton Helmer |
| Chiến lược và cạnh tranh | The Crux | Richard Rumelt |
| Thử nghiệm và suy luận nhân quả | Trustworthy Online Controlled Experiments | Ron Kohavi, Diane Tang, Ya Xu |
| Thử nghiệm và suy luận nhân quả | Field Experiments | Alan S. Gerber, Donald P. Green |
| Thử nghiệm và suy luận nhân quả | Causal Inference for Statistics, Social, and Biomedical Sciences | Guido W. Imbens, Donald B. Rubin |
| Marketing measurement | Marketing Metrics | Neil Bendle, Paul W. Farris, Phillip E. Pfeifer, David J. Reibstein |
| Nghiên cứu định lượng và UX | Quantifying the User Experience | Jeff Sauro, James R. Lewis |
| Nghiên cứu định lượng và UX | Measuring the User Experience | Tom Tullis, Bill Albert |
| Tâm lý và hành vi người tiêu dùng | The Cambridge Handbook of Consumer Psychology | Michael I. Norton, Derek D. Rucker, Cait Lamberton |
| Tâm lý và hành vi người tiêu dùng | Handbook of Consumer Psychology | Curtis P. Haugtvedt, Paul M. Herr, Frank R. Kardes |
| Tâm lý và hành vi người tiêu dùng | Consumer Behavior: Buying, Having, and Being | Michael R. Solomon |
| Pricing | The Strategy and Tactics of Pricing | Thomas T. Nagle và cộng sự |
| Pricing | Pricing Strategy | Tim J. Smith |
| Pricing | Monetizing Innovation | Madhavan Ramanujam, Georg Tacke |
| Tài chính và unit economics | Managerial Accounting | Ray H. Garrison, Eric W. Noreen, Peter C. Brewer |
| Tài chính và unit economics | Financial Intelligence for Entrepreneurs | Karen Berman, Joe Knight |
| Tài chính và unit economics | Principles of Corporate Finance | Richard Brealey, Stewart Myers, Franklin Allen |
| Content writing và cấu trúc thông tin | The Minto Pyramid Principle | Barbara Minto |
| Content writing và cấu trúc thông tin | Style: Lessons in Clarity and Grace | Joseph M. Williams, Joseph Bizup |
| Content writing và cấu trúc thông tin | Content Design | Sarah Richards |
| Content writing và cấu trúc thông tin | The Sense of Style | Steven Pinker |
| Content writing và cấu trúc thông tin | On Writing Well | William Zinsser |
| Forecasting và ra quyết định | Superforecasting | Philip E. Tetlock, Dan Gardner |
| Forecasting và ra quyết định | Noise | Daniel Kahneman, Olivier Sibony, Cass R. Sunstein |
| Forecasting và ra quyết định | The Signal and the Noise | Nate Silver |
| Forecasting và ra quyết định | Thinking in Bets | Annie Duke |
| Quản trị tri thức và tổ chức | Working Knowledge | Thomas H. Davenport, Laurence Prusak |
| Quản trị tri thức và tổ chức | The Knowledge-Creating Company | Ikujiro Nonaka, Hirotaka Takeuchi |
| Quản trị và vận hành | High Output Management | Andrew S. Grove |
| Quản trị và vận hành | The Checklist Manifesto | Atul Gawande |
| Supply chain và chất lượng | Designing and Managing the Supply Chain | David Simchi-Levi và cộng sự |
| Supply chain và chất lượng | Purchasing and Supply Chain Management | Robert M. Monczka và cộng sự |
| Supply chain và chất lượng | Factory Physics | Wallace J. Hopp, Mark L. Spearman |
| Supply chain và chất lượng | Introduction to Statistical Quality Control | Douglas C. Montgomery |
| Supply chain và chất lượng | The Goal | Eliyahu M. Goldratt, Jeff Cox |
| Customer experience và retention | Customer Experience 3.0 | John A. Goodman |
| Customer experience và retention | The Effortless Experience | Matthew Dixon, Nick Toman, Rick DeLisi |
| Nhóm nội dung | Tên sách | Tên tác giả |
|---|---|---|
| Nền tảng hiệu quả quảng cáo | Advertising and Promotion: An Integrated Marketing Communications Perspective | George E. Belch, Michael A. Belch |
| Nền tảng hiệu quả quảng cáo | Effective Advertising: Understanding When, How, and Why Advertising Works | Gerard J. Tellis |
| Chiến lược sáng tạo quảng cáo | Advertising Creative: Strategy, Copy, and Design | Tom Altstiel, Jean Grow, Marcel Jennings |
| Video performance | Ogilvy on Advertising | David Ogilvy |
| Video performance | Tested Advertising Methods | John Caples, Fred Hahn |
| Video performance | Breakthrough Advertising | Eugene M. Schwartz |
| Video performance và thử nghiệm | Trustworthy Online Controlled Experiments | Ron Kohavi, Diane Tang, Ya Xu |
| Đo lường hiệu quả video | Marketing Metrics | Neil Bendle, Paul W. Farris, Phillip E. Pfeifer, David J. Reibstein |
| Video branding | How Brands Grow | Byron Sharp |
| Video branding | How Brands Grow Part 2 | Jenni Romaniuk, Byron Sharp |
| Video branding | Building Distinctive Brand Assets | Jenni Romaniuk |
| Video branding | Strategic Brand Management: Building, Measuring, and Managing Brand Equity | Kevin Lane Keller, Vanitha Swaminathan |
| Video branding | The Long and the Short of It | Les Binet, Peter Field |
| Cấu trúc hình ảnh video | The Visual Story | Bruce Block |
| Ngôn ngữ cảnh quay | Grammar of the Shot | Christopher J. Bowen |
| Nguyên lý dựng video | Grammar of the Edit | Christopher J. Bowen |
| Nguyên lý dựng và nhịp kể | In the Blink of an Eye | Walter Murch |
| Ngôn ngữ điện ảnh | Film Art: An Introduction | David Bordwell, Kristin Thompson, Jeff Smith |
| Âm thanh trong video | Audio-Vision: Sound on Screen | Michel Chion |" trên trình duyệt | `(-1200, 458)` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B30** | `DragDrop` | `chrome.exe` | Bấm [Text] "không nghĩ manual upload phải đi qua approval/review queue ngay từ đầu" trên trình duyệt | `Giao diện UIA` |
| **B31** | `DragDrop` | `chrome.exe` | Bấm [Text] "Mọi thứ upload vào đây đều là knowledge của công ty." trên trình duyệt | `Giao diện UIA` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **16,415** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **79.8%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **10,924 (66.5%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **4.1%** (542 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **93.9%** (12,311 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **1.3%** (167 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **0.6%** (85 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Kiểm tra trạng thái uptime của cụm dịch vụ MinIO (9000), PostgreSQL (5432) và Ingestion Server (8080) hàng ngày.
2. Xem xét và phê duyệt các đề xuất cấp quyền truy cập dữ liệu hoặc điều chỉnh ngân sách quảng cáo trên Lark Approval.
3. Bảo đảm an toàn bảo mật dữ liệu khách hàng và quy trình bí mật kinh doanh của doanh nghiệp.
