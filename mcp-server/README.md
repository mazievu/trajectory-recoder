# TRAJECTORY RECORDER MCP SERVER (DÀNH CHO AI AGENTS)

Hệ thống **Model Context Protocol (MCP) Server** chuyên cung cấp dữ liệu hành vi nhân viên, tiến độ công việc và luồng thao tác máy tính từ **Trajectory Ingestion Server** cho các mô hình AI (Claude Desktop, Google Antigravity, Cursor, ChatGPT, AutoGen,...) trên **máy cục bộ HOẶC BẤT KỲ MÁY NÀO KHÁC TRONG MẠNG**.

---

## 1. TÍNH NĂNG TỐI ƯU HÓA TOKEN CỦA MCP SERVER

1. **Bộ lọc rác tầng Server (Server-Side Noise Reduction)**:
   - Tự động loại bỏ hơn 80% sự kiện cửa sổ ngầm vô hình (`OLEChannelWnd`, `Default IME`, `CicMarshalWnd`, `MSCTF`, `Qt5ClipboardView`).
   - Tự động bỏ qua các tiến trình daemon hệ thống không phải do con người thao tác (`svchost.exe`, `splwow64.exe`, `taskhostw.exe`, `dwm.exe`).
2. **Gộp chuyển động chuột (Mouse Move Aggregation)**:
   - Gom hàng trăm sự kiện `WM_MOUSEMOVE` li ti thành vector chuyển động duy nhất: `(x_start, y_start) -> (x_end, y_end)`.
   - Nếu thao tác di chuột dẫn tới một cú `Click`: Chuỗi di chuột được hấp thụ hoàn toàn vào sự kiện Click.
3. **Gộp chuỗi văn bản gõ (Typing Burst Rollup)**:
   - Gom các ký tự đơn lẻ thành đoạn văn bản hoàn chỉnh.
4. **Siêu tiết kiệm Token (Chỉ ~800 - 1.500 tokens / 1 ngày làm việc)**:
   - Thay vì nạp 50 triệu tokens dữ liệu thô, AI đọc báo cáo trọn vẹn 1 ngày của nhân viên chỉ tốn **dưới 1.000 tokens**.

---

## 2. DANH SÁCH TOOLS CUNG CẤP CHO AI

| Tên Tool | Mô tả chức năng | Token ước tính |
| :--- | :--- | :---: |
| `list_active_machines()` | Liệt kê toàn bộ danh sách máy tính trong công ty, trạng thái ONLINE/OFFLINE, nhịp tim gần nhất và tổng số phiên. | ~100 tokens |
| `get_daily_summary(machine_id, date)` | Xuất báo cáo tổng kết ngày của 1 nhân viên: Dòng thời gian các phân đoạn công việc (Episodes), Top ứng dụng, tài liệu và thời gian nghỉ. | ~800 - 1.500 tokens |
| `inspect_timeline(machine_id, session_id, limit)` | Soi chi tiết từng thao tác click, gõ phím, cuộn trang trong một phiên cụ thể (đã qua bộ lọc rác). | ~300 - 600 tokens |
| `search_actions(keyword, machine_id, date)` | Tìm kiếm từ khóa (tên dự án, file Word, tab Chrome, nội dung gõ) xem nhân viên làm lúc nào. | ~200 - 400 tokens |
| `get_productivity_metrics(machine_id, date)` | Phân tích tỷ lệ sử dụng phần mềm, tần suất chuyển đổi ngữ cảnh (Context Switching). | ~250 tokens |

---

## 3. CÁCH KẾT NỐI TỪ MÁY KHÁC QUA MẠNG (REMOTE / LAN / SSE)

### Bước 1: Khởi chạy MCP Server trên máy chủ 192.168.1.24
Nhấp đúp chuột vào file:
`mcp-server\start_mcp_network.bat`
*(Server sẽ mở cổng lắng nghe mạng: `http://0.0.0.0:8000/sse`)*

### Bước 2: Cấu hình trên máy tính của bạn (hoặc máy bất kỳ khác trong mạng)

#### A. Cấu hình trên Claude Desktop (trên máy khác):
Mở file `%APPDATA%\Claude\claude_desktop_config.json` trên máy đó:
```json
{
  "mcpServers": {
    "trajectory": {
      "url": "http://192.168.1.24:8000/sse"
    }
  }
}
```

#### B. Cấu hình trên Cursor / Windsurf (trên máy khác):
Trong phần Settings -> Features -> MCP Servers, chọn **Add New MCP Server**:
- **Name**: `trajectory`
- **Type**: `SSE`
- **Server URL**: `http://192.168.1.24:8000/sse`

#### C. Cấu hình trên Antigravity / AI Agent khác:
```json
{
  "name": "trajectory",
  "url": "http://192.168.1.24:8000/sse"
}
```

---

## 4. CÁCH KẾT NỐI CỤC BỘ TRÊN CHÍNH MÁY CHỦ (STDIO MODE)
Nếu AI chạy trên cùng máy tính lưu dữ liệu:
```json
{
  "mcpServers": {
    "trajectory": {
      "command": "python",
      "args": [
        "D:\\tools GTF\\trajectory recoder\\mcp-server\\server.py",
        "--transport", "stdio"
      ]
    }
  }
}
```

---

## 5. KIỂM THỬ ĐỘC LẬP
Kiểm tra hoạt động của tất cả 5 tools:
```powershell
python mcp-server/test_mcp.py
```
