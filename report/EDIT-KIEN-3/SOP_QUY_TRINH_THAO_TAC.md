# QUY TRÌNH THAO TÁC CHUẨN (SOP) CHI TIẾT THEO TỪNG NÚT BẤM
**Mã máy trạm**: `EDIT-KIEN-3`  
**Vai trò đảm nhiệm**: **Render Station / Secondary Video Editor**  
**Phòng ban / Bộ phận**: Video Production & Batch Processing  
**Thời điểm chuẩn hóa**: 2026-09-23 23:07:01 (Giờ VN)  
**Công nghệ chuẩn hóa**: `Laya Multi-Stage QC Engine (CUDA Accelerated)`  
**Căn cứ dữ liệu**: **5,653** thao tác ghi nhận thực tế (4,633 thao tác nghiệp vụ, đã khử 4,126 lỗi lệch context).  

---

## 1. MỤC TIÊU & TỔNG QUAN TÁC VỤ CỐT LÕI
- **Mô tả công việc**: Dựng video phụ trợ, xử lý hiệu ứng hàng loạt, render project nặng giảm tải cho trạm chính.
- **Bộ phần mềm tác nghiệp chính**: `CapCut.exe`, `chrome.exe`, `explorer.exe`.
- **Định mức chu kỳ hoàn thành**: `20 - 35 phút / batch render`.
- **Tổ hợp phím tắt khuyến nghị**: `Ctrl+B` (Split), `Ctrl+E` (Export), `Alt+Drag` (Duplicate clip).

---

## 2. BẢNG QUY TRÌNH THAO TÁC CHUẨN (STANDARD OPERATING PROCEDURE - SOP)

### 📍 PHASE 1: CHUẨN BỊ & NHẬP LIỆU (INPUT & ASSET INGESTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B01** | `Click` | `System` | Bấm [ListItem] "4-1. Vid test hàng ở kho" trên phần mềm System | `(1054, 409)` |
| **B02** | `DragDrop` | `chrome.exe` | Tương tác trên trang web: 'Downloads - File Explorer...' | `Giao diện UIA` |
| **B03** | `Click` | `chrome.exe` | Bấm [Button] "Thu nhỏ Văn bản" trên trình duyệt | `(1261, 104)` |
| **B04** | `DragDrop` | `System` | Thao tác DragDrop trên ứng dụng System | `Giao diện UIA` |

### 📍 PHASE 2: XỬ LÝ & THAO TÁC CHÍNH (CORE EDITING / EXECUTION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B05** | `Click` | `chrome.exe` | Bấm [Text] "@" trên trình duyệt | `(-1469, 112)` |
| **B06** | `Click` | `explorer.exe` | Chọn tệp tin / asset trong Windows Explorer | `(926, 1051)` |
| **B07** | `Click` | `chrome.exe` | Tương tác trên trang web: 'Phạm Bích Ngọc - Portfolio video editor - Goo...' | `(1634, 599)` |
| **B08** | `Click` | `chrome.exe` | Bấm [Button] "Thẻ mới" trên trình duyệt | `(351, 16)` |
| **B09** | `Click` | `chrome.exe` | Bấm [ComboBox] "Chế độ Hỏi AI" trên trình duyệt | `(369, 51)` |
| **B10** | `TypeText` | `chrome.exe` | Bấm [Edit] "Thanh địa chỉ và tìm kiếm" trên trình duyệt | `Giao diện UIA` |
| **B11** | `Click` | `chrome.exe` | Bấm [Group] "GREEN (1)" trên trình duyệt | `(796, 444)` |
| **B12** | `Click` | `chrome.exe` | Bấm [Edit] "Search in Drive" trên trình duyệt | `(507, 124)` |
| **B13** | `TypeText` | `CapCut.exe` | Gõ nội dung văn bản (Phụ đề, Tiêu đề kịch bản, Text hiệu ứng) trên CapCut | `Giao diện UIA` |
| **B14** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (410, 322): Khu Vực [Thư viện tài nguyên / Danh sách asset import / Hiệu ứng] | `(410, 322)` |
| **B15** | `Click` | `chrome.exe` | Bấm [DataItem] "CLIP HOÀN Shared folder More info (Alt + →)" trên trình duyệt | `(398, 367)` |
| **B16** | `DoubleClick` | `chrome.exe` | Bấm [DataItem] "CLIP HOÀN Shared folder More info (Alt + →)" trên trình duyệt | `(398, 367)` |
| **B17** | `Click` | `chrome.exe` | Bấm [Hyperlink] "TIKTOK GREENAIR" trên trình duyệt | `(471, 181)` |
| **B18** | `Click` | `chrome.exe` | Bấm [Button] "TIKTOK GREENAIR" trên trình duyệt | `(711, 182)` |
| **B19** | `Click` | `chrome.exe` | Bấm [Hyperlink] "Elehouse video" trên trình duyệt | `(519, 190)` |

### 📍 PHASE 3: XUẤT BẢN & ĐÓNG GÓI (EXPORT & RENDERING)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B20** | `DoubleClick` | `CapCut.exe` | Nhấp đúp chuột (637, 398): Mở chi tiết thuộc tính hoặc edit text trên Màn Hình Preview [Khung xem trước video & Căn chỉnh bounding box khung hình] | `(637, 398)` |
| **B21** | `Click` | `chrome.exe` | Bấm [Group] "Bích Ngọc Phạm" trên trình duyệt | `(411, 272)` |
| **B22** | `Click` | `dllhost.exe` | Bấm [ListItem] "7. NHỚ ĐEM QUÀ CHO KHÁCH " trên phần mềm dllhost.exe | `(1072, 239)` |
| **B23** | `DragDrop` | `chrome.exe` | Bấm [Text] "A on prostoy (Yunosha)
Iz Baku kholostoy (Ya vash)
Kvartira yest' dorogoy (On nash)
Molodoy uzhe tsentrovoy (Oy-yo-yo-yoy)

[Verse 1: MIA BOYKA]
Vyglyadit, kak papa (O)
No po faktu ventilyator (O)
Yakhta na primete (U)
Sil'no duyet veter

[Pre-Chorus: SABI & MIA BOYKA]
Mozgi ne kruti
Fakty primi
Skol'ko ne govori
Zakon dlya vsekh odin

[Chorus: Sabi & MIA BOYKA]
IPhone kupi, restoran oplati
Bez povoda mne podari
Kol'tso s moim imenem
Bazovyy minimum
Skuchay, bez povoda obnimay (Ay)
Lyubi, tseni, uvazhay (Ay)
Chuvstva samye sil'nye
Eto bazovyy minimum

[Bridge: Sabi & MIA BOYKA]
Eto bazovyy minimum, ay
Eto bazovyy minimum, bay-bay
Eto bazovyy minimum, ay
Eto bazovyy minimum

[Verse 2: MIA BOYKA]
Igrayet v gol'f s okhranoy
Takoy ser'yoznyy paren'
Uchilsya za granitsey
Ochen' bogatyy mister

[Verse 3: Sabi]
Mamin syn i tol'ko
Dlya neyo podarok
Ochen' mnogo loro (Tak mnogo)
Postoyanno p'yano

[Pre-Chorus: SABI & MIA BOYKA]
Mozgi ne kruti
Fakty primi
Skol'ko ne govori
Zakon dlya vsekh odin

[Chorus: Sabi & MIA BOYKA]
IPhone kupi, restoran oplati
Bez povoda mne podari
Kol'tso s moim imenem
Bazovyy minimum
Skuchay, bez povoda obnimay (Ay)
Lyubi, tseni, uvazhay (Ay)
Chuvstva samye sil'nye
Eto bazovyy minimum
[Outro: Sabi & MIA BOYKA]
Eto bazovyy minimum, ay
Eto bazovyy minimum, bay-bay
Eto bazovyy minimum, ay
Eto bazovyy minimum" trên trình duyệt | `Giao diện UIA` |
| **B24** | `Click` | `chrome.exe` | Bấm [Text] "Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy
Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy

Tselyy den' goryu v ogne
Tselyy den' ishchu otvet
Ty ushla, no ved' vremya lechit
Pomnyu nash posledniy vecher
Vse zabiray moya beybi
Vse zabiray moya beybi
Ne vybirayem moment
YA dopivayu portveyn
Ty postoy
Ne idi za mnoy
Vse tot slova prichinyayu bol'


Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy
Teper' nochami pod lunoy

Obnimat' budet drugoy
Ne ugnat'sya za toboy" trên trình duyệt | `(158, 441)` |
| **B25** | `DragDrop` | `chrome.exe` | Bấm [Text] "Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy
Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy

Tselyy den' goryu v ogne
Tselyy den' ishchu otvet
Ty ushla, no ved' vremya lechit
Pomnyu nash posledniy vecher
Vse zabiray moya beybi
Vse zabiray moya beybi
Ne vybirayem moment
YA dopivayu portveyn
Ty postoy
Ne idi za mnoy
Vse tot slova prichinyayu bol'


Teper' nochami pod lunoy
Obnimat' budet drugoy
Ne ugnat'sya za toboy
Teper' nochami pod lunoy

Obnimat' budet drugoy
Ne ugnat'sya za toboy" trên trình duyệt | `Giao diện UIA` |
| **B26** | `Click` | `chrome.exe` | Bấm [Text] "A cho bọn e xin demo để bọn e nghiên cứu content với ạ" trên trình duyệt | `(-1410, 607)` |
| **B27** | `Click` | `chrome.exe` | Bấm [Button] "Điều kiện kích hoạt cài đặt" trên trình duyệt | `(1147, 970)` |
| **B28** | `Click` | `chrome.exe` | Bấm [Button] "Bắt đầu tạo" trên trình duyệt | `(1241, 975)` |
| **B29** | `Click` | `chrome.exe` | Bấm [Button] "Tuỳ chọn khác" trên trình duyệt | `(575, 216)` |
| **B30** | `Click` | `dllhost.exe` | Bấm [ListItem] "Đổi_mặt_nhân_vật_trong_20260915142335" trên phần mềm dllhost.exe | `(242, 223)` |
| **B31** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1449, 506): Thanh Điều Khiển Preview [Nút Play / Pause / Tua Timeline / Fullscreen] | `(1449, 506)` |
| **B32** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1095, 434): Màn Hình Preview [Khung xem trước video & Căn chỉnh bounding box khung hình] | `(1095, 434)` |
| **B33** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (770, 507): Thanh Điều Khiển Preview [Nút Play / Pause / Tua Timeline / Fullscreen] | `(770, 507)` |
| **B34** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (802, 689): Vùng Timeline Cắt Ghép [Track Video chính / Track Phụ đề / Track Âm thanh / Kéo thả clip] | `(802, 689)` |

### 📍 PHASE 4: BÀN GIAO & LƯU TRỮ (DELIVERY & COLLABORATION)

| Bước | Hành động | Phần mềm (Chuẩn hóa) | Chi tiết thao tác & Nút bấm / Vùng tương tác | Tọa độ Pixel (X, Y) |
| :---: | :--- | :--- | :--- | :--- |
| **B35** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (377, 383): Khu Vực [Thư viện tài nguyên / Danh sách asset import / Hiệu ứng] | `(377, 383)` |
| **B36** | `DoubleClick` | `CapCut.exe` | Nhấp đúp chuột (377, 383): Mở chi tiết thuộc tính hoặc edit text trên Khu Vực [Thư viện tài nguyên / Danh sách asset import / Hiệu ứng] | `(377, 383)` |
| **B37** | `DoubleClick` | `System` | Bấm [ListItem] "1. Vid máy làm sữa hạt_" trên phần mềm System | `(1492, 407)` |
| **B38** | `Click` | `chrome.exe` | Bấm [DataItem] "TÀI NGUYÊN Shared folder More info (Alt + →)" trên trình duyệt | `(637, 398)` |
| **B39** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (429, 384): Khu Vực [Thư viện tài nguyên / Danh sách asset import / Hiệu ứng] | `(429, 384)` |
| **B40** | `DoubleClick` | `CapCut.exe` | Nhấp đúp chuột (429, 384): Mở chi tiết thuộc tính hoặc edit text trên Khu Vực [Thư viện tài nguyên / Danh sách asset import / Hiệu ứng] | `(429, 384)` |
| **B41** | `Click` | `chrome.exe` | Bấm [Text] "
Trans: Zenyu
♫-----------------------------------------------♫
Lời bài hát:
渔火夜风陋窗
无意吹开那卷那章
绘着陈塘重浪
雷闪中剑刎项
双瞳无泪无光
桧柏叶落恩怨难偿
四海水龙共庆
风未止魂飘荡
三载六月孕得丑时而降
留印象满屋绽荷香
天生天赐千七百戒杀相
父子情越裂越伤
他本是一世无双
太子位沉檀凝香
东海之畔捉龙回浪
他合眼一世悲伤
乾元山描尽风霜
风火轮又沸乱血浆
喧夏曛曛漾漾
弓马半跨 箭轨勾光
穿过太虚云裳
却酿下劫一场
莲花刻他模样
至性嚣张味火则狂
烧尽龙宫汪洋
方松开手中枪
三载六月孕得丑时而降
留印象满屋绽荷香
天生天赐千七百戒杀相
父子情越裂越伤
他本是一世无双
太子位沉檀凝香
东海之畔捉龙回浪
他合眼一世悲伤
乾元山描尽风霜
风火轮又沸乱血浆
他本是一世无双
踏着风混天绫响
红莲重生血脉相向
他肩扛紫焰尖枪
浓眉上写着沧桑
乾坤圈藏一滴泪光
他本是一世无双
踏着风混天绫响
红莲重生血脉相向
他肩扛紫焰尖枪
浓眉上写着沧桑
乾坤圈藏一滴泪光
半卷书尽了他心伤
yúhuǒ yè fēng lòu chuāng
wúyì chuī kāi nà juǎn nà zhāng
huìzhe chén táng zhòng làng
léi shǎn zhōng jiàn wěn xiàng
shuāng tóng wú lèi wú guāng
guì bǎi yè luò ēnyuàn nán cháng
sì hǎishuǐ lóng gòng qìng
fēng wèi zhǐ hún piāodàng
sān zài liù yuè yùn dé chǒu shí'ér jiàng
liú yìnxiàng mǎn wū zhàn hé xiāng
tiānshēng tiāncì qiān qībǎi jiè shā xiāng
fùzǐ qíng yuè liè yuè shāng
tā běn shì yīshì wúshuāng
tàizǐ wèi chén tánníngxiāng
dōnghǎi zhī pàn zhuō lóng huí làng
tā héyǎn yīshì bēishāng
qián yuán shān miáo jǐn fēngshuāng
fēng huǒ lún yòu fèi luàn xiějiāng
xuān xià xūn xūn yàng yàng
gōng mǎ bàn kuà jiàn guǐ gōu guāng
chuānguò tài xū yún shang
què niàng xià jié yī chǎng
liánhuā kè tā múyàng
zhì xìng xiāozhāng wèi huǒ zé kuáng
shāo jǐn lónggōng wāngyáng
fāng sōng kāi shǒuzhōng qiāng
sān zài liù yuè yùn dé chǒu shí'ér jiàng
liú yìnxiàng mǎn wū zhàn hé xiāng
tiānshēng tiāncì qiān qībǎi jiè shā xiāng
fùzǐ qíng yuè liè yuè shāng
tā běn shì yīshì wúshuāng
tàizǐ wèi chén tánníngxiāng
dōnghǎi zhī pàn zhuō lóng huí làng
tā héyǎn yīshì bēishāng
qián yuán shān miáo jǐn fēngshuāng
fēng huǒ lún yòu fèi luàn xiějiāng
tā běn shì yīshì wúshuāng
tàzhe fēng hùn tiān líng xiǎng
hóng lián chóngshēng xuèmài xiāngxiàng
tā jiān káng zǐ yàn jiān qiāng
nóngméi shàng xiězhe cāngsāng
qiánkūn quān cáng yīdī lèi guāng
tā běn shì yīshì wúshuāng
tàzhe fēng hùn tiān líng xiǎng
hóng lián chóngshēng xuèmài xiāngxiàng
tā jiān káng zǐ yàn jiān qiāng
nóngméi shàng xiězhe cāngsāng
qiánkūn quān cáng yīdī lèi guāng
bàn juǎn shū jǐnle tā xīn shāng
♫-----------------------------------------------♫
Email contact: zenyunichen@gmail.com
♫-----------------------------------------------♫
Cảm ơn mọi người đã nghe ủng hộ Zen!






" trên trình duyệt | `(177, 977)` |
| **B42** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1886, 169): Bảng Thuộc Tính (Inspector) [Chỉnh Video / Âm thanh / Tốc độ / Hoạt ảnh / Màu sắc] | `(1886, 169)` |
| **B43** | `Click` | `CapCut.exe` | Nhấp chuột tại tọa độ (1836, 164): Bảng Thuộc Tính (Inspector) [Chỉnh Video / Âm thanh / Tốc độ / Hoạt ảnh / Màu sắc] | `(1836, 164)` |
| **B44** | `DragDrop` | `Lark.exe` | Bấm "đổi mặt nhân vật trong vid này sang mặt một người phụ nữ da ngăm đen hơi mũm mĩm một chút, má bánh bao phính phính còn lại giữ nguyên toàn bộ các cảnh, thiết kế của trang phục, effect và chữ như vid gốc" trên ứng dụng Lark để gửi thông báo/nộp kết quả | `Giao diện UIA` |
| **B45** | `DragDrop` | `CapCut.exe` | Bấm nút [Text] "Thả nội dung nghe nhìn" trên giao diện CapCut | `Giao diện UIA` |
| **B46** | `Click` | `chrome.exe` | Bấm [Button] "Thêm thành phần vào ô nhập câu lệnh" trên trình duyệt | `(684, 971)` |

---

## 3. CHỈ SỐ ĐỊNH MỨC HIỆU SUẤT & THỰC ĐO LAYA QC (BENCHMARK METRICS)

| Chỉ số đo lường | Số liệu thực tế ghi nhận | Ý nghĩa & Đánh giá |
| :--- | :---: | :--- |
| **Tổng thao tác ghi nhận** | **5,653** | Khối lượng tương tác tổng thể trong các ca làm việc |
| **Tỷ lệ đúng việc (Work Efficiency)** | **82.0%** | Đo lường mức độ tập trung chuyên môn, đã loại bỏ tác vụ ngoài |
| **Độ sạch dữ liệu (Mismatches Reconciled)** | **4,126 (73.0%)** | Tỷ lệ lỗi cửa sổ ảo đã được Laya tự động nắn chuẩn |
| **Phân bổ thời gian: Chuẩn bị (Phase 1)** | **1.2%** (54 acts) | Thời gian tìm asset, đọc kịch bản và chuẩn bị file |
| **Phân bổ thời gian: Thao tác chính (Phase 2)** | **95.6%** (4,431 acts) | Thời lượng thao tác trọng tâm trên phần mềm nghiệp vụ |
| **Phân bổ thời gian: Xuất bản (Phase 3)** | **2.2%** (103 acts) | Tiến trình render, đóng gói, lưu dự án |
| **Phân bổ thời gian: Bàn giao (Phase 4)** | **1.0%** (45 acts) | Trao đổi nội bộ, upload Drive, nhắn tin báo cáo hoàn thành |

---

## 4. QUY TẮC AN TOÀN & BẢO ĐẢM TÍNH LIÊN TỤC CỦA DỰ ÁN
1. Đồng bộ hóa thư mục asset nguồn từ NAS hoặc Google Drive dùng chung trước khi khởi chạy render.
2. Kiểm tra hàng đợi render: Không mở quá 2 tiến trình render song song để tránh sụt giảm hiệu năng GPU.
3. Đóng gói project folder và dọn dẹp file temp/proxy sau khi bàn giao thành phẩm.
