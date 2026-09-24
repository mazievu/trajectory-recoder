# BÁO CÁO CẤU HÌNH PHẦN CỨNG DÀN MÁY TRẠM (GTF LAN)
**Thời điểm quét**: 2026-09-22 13:56:21  
**Máy chủ quét**: NGOCMINH-PC (192.168.1.24)  

---

## 1. BẢNG CHI TIẾT CẤU HÌNH PHẦN CỨNG

| Máy trạm | IP LAN | Trạng thái | Vi xử lý (CPU) | Tổng RAM (GB) | Card Đồ họa (GPU) | Dung lượng Ổ đĩa | Phương thức |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **NGOCMINH-PC** | 127.0.0.1 (192.168.1.24) | Online | Intel(R) Core(TM) i7-14700K | **31.8** | NVIDIA GeForce RTX 4060 Ti, Intel(R) UHD Graphics 770 | C: 47.9GB/499.1GB | D: 748.9GB/931.5GB | E: 898.5GB/931.5GB | F: 267.2GB/430.6GB | Local WMI |
| **MR-QUAN** | 192.168.1.18 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **EDIT-KIEN-2** | 192.168.1.26 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **EDIT-KIEN-3** | Unknown | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **EDIT-NGOCMINH** | 192.168.1.25 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **DESKTOP-1Q714D7** | Unknown | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **DESKTOP-3F6NKQA** | 192.168.1.10 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **DESKTOP-K5AMUHI** | Unknown | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **DESKTOP-MCV5QUC** | 192.168.1.19 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **DESKTOP-VO7GFI0** | Unknown | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **GTF-22** | 192.168.1.13 | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |
| **TESTER-PC** | Unknown | No Ping Response | Lỗi: The WinRM client cannot process the request. If the authenti... | **N/A** | N/A | N/A | None |

---

## 2. HƯỚNG DẪN BỔ SUNG NẾU MÁY BÁO 'ACCESS DENIED'
Trong mạng Windows Workgroup (không gia nhập Domain Active Directory), Windows mặc định chặn truy vấn WMI từ máy khác nếu không cung cấp User/Password có quyền Administrator trên máy đó.

1. **Nếu bạn có mật khẩu Admin chung**: Chạy lệnh kèm tham số `-Credential`:
   `.\Scan-LanHardware.ps1 -Credential (Get-Credential)`
2. **Giải pháp 100% không cần mật khẩu**: Gửi tệp `Collect-Silent.vbs` cho nhân viên nhấp đúp (hoặc chạy qua USB/Lark). Script sẽ tự chạy ngầm 0.3s không hiện cửa sổ và gửi thông số về máy chủ.