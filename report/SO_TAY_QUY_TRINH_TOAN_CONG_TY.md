# SỔ TAY QUY TRÌNH VẬN HÀNH TOÀN CÔNG TY (ENTERPRISE MASTER SOP)
**Ngày phát hành chuẩn hóa**: 2026-09-23  
**Phạm vi áp dụng**: Toàn bộ 12 vị trí máy trạm tại trụ sở GTF  
**Căn cứ kiểm định**: Đã qua kiểm định và chuẩn hóa bởi `Laya Multi-Stage QC Engine` trên GPU RTX 4060 Ti  
**Quy mô dữ liệu thực tế**: **673 sessions**, **366,394 thao tác** (Khử thành công **201,267** lỗi lệch context)  

---

## 1. SƠ ĐỒ DÒNG CHẢY CÔNG VIỆC PHỐI HỢP LIÊN PHÒNG BAN (CROSS-FUNCTIONAL WORKFLOW)

```mermaid
flowchart TD
    subgraph Research['1. Khâu Nghiên Cứu & Lên Kịch Bản']
        A['DESKTOP-MCV5QUC<br/>(Content Strategist)'] -->|Kịch bản & Brief trên Lark Docs| B['DESKTOP-1Q714D7<br/>(Graphic Designer)']
    end

    subgraph Creative['2. Khâu Thiết Kế & Biên Tập Video']
        B -->|Asset Banner/Vector| C['EDIT-KIEN-2 & EDIT-KIEN-3<br/>(Video Editors)']
        A -->|Brief Video Nail Art/Family| D['DESKTOP-3F6NKQA & VO7GFI0<br/>(Specialized Creators)']
        C --> E['MR-QUAN<br/>(Lead Producer Review & Duyệt)']
        D --> E
    end

    subgraph Growth['3. Khâu Quảng Cáo & Phân Phối']
        E -->|Video Master Đã Duyệt| F['DESKTOP-K5AMUHI<br/>(TikTok Ads Media Buyer)']
        E -->|Video Master Đã Duyệt| G['GTF-22<br/>(Ads Operations & Scaling)']
    end

    subgraph Management['4. Khâu Quản Trị & Hạ Tầng Kỹ Thuật']
        F & G -->|Báo Cáo Hiệu Quả & ROAS| H['NGOCMINH-PC & EDIT-NGOCMINH<br/>(Management & Tech Lead)']
        I['TESTER-PC<br/>(QA & System Test)'] -.-> H
    end
```

---

## 2. MA TRẬN PHÂN CÔNG VÀ CHỈ SỐ CHUẨN HÓA LAYA QC (12 MÁY TRẠM)

| STT | Mã Máy Trạm | Chức danh / Vai trò | Tổng Thao Tác | Tỷ lệ Đúng Việc | Phân Bổ (P1 / P2 / P3 / P4) | Tài liệu SOP Chi Tiết |
| :---: | :--- | :--- | :---: | :---: | :---: | :--- |
| 1 | **`DESKTOP-K5AMUHI`** | Media Buyer (TikTok Ads Specialist) | **63,562** | 🟢 **79.7%** | `3.7% / 93.2% / 1.6% / 1.4%` | [DESKTOP-K5AMUHI SOP](./DESKTOP-K5AMUHI/SOP_QUY_TRINH_THAO_TAC.md) |
| 2 | **`MR-QUAN`** | Lead Video Producer (Quan - Tổng duyệt & Sản xuất) | **60,735** | 🟢 **91.1%** | `0.6% / 63.1% / 36.1% / 0.3%` | [MR-QUAN SOP](./MR-QUAN/SOP_QUY_TRINH_THAO_TAC.md) |
| 3 | **`EDIT-NGOCMINH`** | Tech Lead / AI Developer Workstation | **46,740** | 🟢 **78.9%** | `2.1% / 96.3% / 1.4% / 0.3%` | [EDIT-NGOCMINH SOP](./EDIT-NGOCMINH/SOP_QUY_TRINH_THAO_TAC.md) |
| 4 | **`EDIT-KIEN-2`** | Senior Video Editor (Kien - Trạm dựng chính) | **40,439** | 🟢 **83.0%** | `4.6% / 92.9% / 1.9% / 0.5%` | [EDIT-KIEN-2 SOP](./EDIT-KIEN-2/SOP_QUY_TRINH_THAO_TAC.md) |
| 5 | **`DESKTOP-MCV5QUC`** | Content Strategist & Creative Researcher | **38,607** | 🟢 **80.1%** | `4.5% / 92.1% / 2.8% / 0.7%` | [DESKTOP-MCV5QUC SOP](./DESKTOP-MCV5QUC/SOP_QUY_TRINH_THAO_TAC.md) |
| 6 | **`DESKTOP-1Q714D7`** | Graphic & Vector Designer | **27,002** | 🟡 **60.6%** | `5.5% / 83.7% / 10.5% / 0.3%` | [DESKTOP-1Q714D7 SOP](./DESKTOP-1Q714D7/SOP_QUY_TRINH_THAO_TAC.md) |
| 7 | **`DESKTOP-3F6NKQA`** | Specialized Video Editor (Nail Art Niche) | **26,304** | 🟢 **80.2%** | `2.9% / 95.5% / 1.0% / 0.7%` | [DESKTOP-3F6NKQA SOP](./DESKTOP-3F6NKQA/SOP_QUY_TRINH_THAO_TAC.md) |
| 8 | **`GTF-22`** | Marketing & Ads Operations Specialist | **18,666** | 🟢 **84.2%** | `5.4% / 89.6% / 4.2% / 0.8%` | [GTF-22 SOP](./GTF-22/SOP_QUY_TRINH_THAO_TAC.md) |
| 9 | **`DESKTOP-VO7GFI0`** | Video Creator (Baby Carrier & Family Products) | **18,503** | 🟢 **83.0%** | `0.7% / 90.6% / 7.3% / 1.4%` | [DESKTOP-VO7GFI0 SOP](./DESKTOP-VO7GFI0/SOP_QUY_TRINH_THAO_TAC.md) |
| 10 | **`NGOCMINH-PC`** | Management / Server & Systems Operations | **16,415** | 🟢 **79.8%** | `4.1% / 93.9% / 1.3% / 0.6%` | [NGOCMINH-PC SOP](./NGOCMINH-PC/SOP_QUY_TRINH_THAO_TAC.md) |
| 11 | **`EDIT-KIEN-3`** | Render Station / Secondary Video Editor | **5,653** | 🟢 **82.0%** | `1.2% / 95.6% / 2.2% / 1.0%` | [EDIT-KIEN-3 SOP](./EDIT-KIEN-3/SOP_QUY_TRINH_THAO_TAC.md) |
| 12 | **`TESTER-PC`** | QA / Testing & Infrastructure Verification | **3,768** | 🟢 **81.8%** | `0.6% / 83.1% / 15.4% / 0.8%` | [TESTER-PC SOP](./TESTER-PC/SOP_QUY_TRINH_THAO_TAC.md) |

**Định mức trung bình toàn công ty**: Tỷ lệ làm việc đúng chuyên môn đạt **81.0%**.

---

## 3. CÁC QUY CHUẨN ĐỒNG BỘ LIÊN PHÒNG BAN (STANDARD HANDOFF RULES)

### 3.1 Giao diện Bàn giao giữa Content $\rightarrow$ Design & Video Editing:
- **Công cụ**: Sử dụng **Lark Docs** đính kèm bảng Storyboard chi tiết từng cảnh quay.
- **Tài nguyên**: Mọi file ảnh/video thô phải được đưa vào thư mục chung `OpenClaw-Knowledge` hoặc Google Drive nội bộ.

### 3.2 Giao diện Bàn giao giữa Video Editing $\rightarrow$ Lead Producer Review:
- **Định dạng xuất**: MP4, H.264, Bitrate 15-20 Mbps, Audio AAC 320kbps.
- **Tên file chuẩn**: `[YYMMDD]_[Product]_[EditorName]_[V1/V2].mp4`.
- **Trạng thái duyệt**: Lead Producer duyệt trực tiếp trên Lark Base hoặc thư mục duyệt trước khi chuyển sang Media Buyer.

### 3.3 Giao diện Bàn giao giữa Lead Producer $\rightarrow$ Media Buyer / Ads Ops:
- **Thời gian bàn giao**: Chậm nhất 16h00 hàng ngày để kịp setup camp tối và sáng hôm sau.
- **Thông số quảng cáo**: Đính kèm Caption đề xuất, Hook text 3 giây đầu và tệp nhạc bản quyền cho phép.

---

## 4. QUY TRÌNH QUẢN TRỊ RỦI RO & BẢO MẬT HẠ TẦNG DỮ LIỆU
1. **Kiểm soát thông tin nội bộ**: Tuyệt đối không paste token, API key hoặc mật khẩu vào các công cụ AI công cộng.
2. **Bảo toàn dữ liệu máy chủ**: Cơ sở dữ liệu Trajectory Recorder và MinIO Bucket chạy sao lưu tự động hàng tuần.
3. **Cập nhật quy trình liên tục**: Các chỉ số SOP sẽ được Laya Multi-Stage QC tự động cập nhật theo chu kỳ định kỳ.
