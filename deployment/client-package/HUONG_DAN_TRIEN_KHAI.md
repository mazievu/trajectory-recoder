# HƯỚNG DẪN TRIỂN KHAI TRAJECTORY RECORDER (BẢN PRODUCTION)

Tài liệu hướng dẫn cài đặt và triển khai hệ thống **Trajectory Recorder** trên toàn bộ máy tính Windows trong doanh nghiệp.

---

## 1. THÀNH PHẦN BỘ CÀI ĐẶT

Thư mục `production-package` bao gồm các tệp tin:

| Tên tệp | Vai trò | Mô tả |
| :--- | :--- | :--- |
| `trajectory-agent.exe` | Ứng dụng chính | Thu thập sự kiện thao tác (bàn phím, chuột, cửa sổ) |
| `trajectory-uploader.exe` | Ứng dụng tải lên | Mã hóa, nén Zstd, gửi Heartbeat và tải phiên về Server |
| `ca.crt` | Chứng chỉ CA | Xác thực kết nối TLS/HTTPS an toàn với Server 192.168.1.24 |
| `install_autostart.bat` | Trình cài đặt 1-Click | Tự nhận diện Tên máy/User, đăng ký Windows Autostart và kích hoạt chạy ngầm |
| `start_silent.vbs` | Khởi chạy ngầm | Kích hoạt cả 2 tiến trình ở chế độ ẩn hoàn toàn (`WindowStyle = 0`) |
| `register_autostart.vbs` | Đăng ký tự khởi động | Ghi registry `HKCU\...\Run` và tạo shortcut Startup |
| `deploy_silent.ps1` | Triển khai tự động IT | Kịch bản PowerShell dành cho IT cài đặt hàng loạt qua GPO / SCCM / Intune |
| `stop_background.bat` | Dừng ứng dụng | Tắt ngay lập tức các tiến trình đang chạy ngầm |
| `uninstall_autostart.bat` | Gỡ cài đặt | Xóa tự khởi động và tắt hoàn toàn ứng dụng |
| `spool/` | Thư mục bộ đệm | Lưu trữ dữ liệu tạm thời trước khi mã hóa và gửi lên Server |

---

## 2. PHƯƠNG ÁN 1: CÀI ĐẶT NHANH 1-CLICK CHO NHÂN VIÊN

Dành cho người dùng tự cài đặt hoặc IT thao tác trực tiếp trên máy:

1. **Giải nén** tệp `production-package.zip` vào một thư mục cố định trên máy (ví dụ: `C:\Tools\TrajectoryRecorder` hoặc `D:\TrajectoryRecorder`).
2. **Nhấp đúp chuột vào tệp `install_autostart.bat`**.
3. Màn hình cài đặt sẽ tự động:
   - Nhận diện Tên máy (`%COMPUTERNAME%`) và Tên tài khoản (`%USERNAME%`).
   - Đăng ký hệ thống tự khởi động cùng Windows (Registry & Startup).
   - Khởi động ngầm cả 2 ứng dụng `trajectory-agent.exe` và `trajectory-uploader.exe`.
   - Thông báo `[THANH CONG] CAI DAT HOAN TAT!`.
4. **Nhấn phím bất kỳ để đóng cửa sổ.** 
   - Kể từ thời điểm này, ứng dụng sẽ chạy ngầm 100%, không hiện cửa sổ, không popup, và tự động chạy lại mỗi khi bật máy.

---

## 3. PHƯƠNG ÁN 2: TRIỂN KHAI HÀNG LOẠT DÀNH CHO IT SYSADMIN

Dành cho quản trị mạng triển khai đồng loạt từ xa qua GPO (Group Policy), SCCM, Intune hoặc PowerShell Remoting:

### Cách 1: Triển khai qua lệnh PowerShell từ xa
Copy thư mục `production-package` vào máy trạm (ví dụ `C:\ProgramData\TrajectoryRecorder`), sau đó chạy lệnh:
```powershell
powershell.exe -ExecutionPolicy Bypass -File "C:\ProgramData\TrajectoryRecorder\deploy_silent.ps1"
```
Kết quả trả về định dạng JSON kiểm toán:
```json
{"Status":"SUCCESS","MachineId":"DESKTOP-ABC1234","UserId":"nguyenvana","AgentRunning":true,"UploaderRunning":true}
```

### Cách 2: Triển khai với thông số tùy chỉnh
Nếu IT muốn chỉ định mã định danh máy hoặc người dùng theo chuẩn nội bộ:
```powershell
powershell.exe -ExecutionPolicy Bypass -File "deploy_silent.ps1" -MachineId "HR-PC-01" -UserId "hr_user" -ServerUrl "https://192.168.1.24"
```

---

## 4. CƠ CHẾ HOẠT ĐỘNG & TÍNH NĂNG TỰ ĐỘNG BẢO VỆ

- **Chạy ẩn hoàn toàn (100% Silent):** Khởi chạy với `WindowStyle = 0` qua Windows Script Host, không gây hiện tượng chớp nháy màn hình cmd, không có icon dưới khay hệ thống, không làm gián đoạn công việc của nhân viên.
- **Tự động phục hồi (Self-Healing Supervisor):** `trajectory-agent.exe` liên tục giám sát `trajectory-uploader.exe`. Nếu tiến trình tải lên vô tình bị tắt, agent sẽ tự động khởi chạy lại sau 5 giây.
- **Chống chạy đè (Single-Instance Mutex):** Sử dụng Named Mutex `Local\TrajectoryUploaderSingleInstance`, đảm bảo không bao giờ bị chạy nhân đôi tiến trình.
- **Bảo toàn dữ liệu khi mất mạng:** Nếu mất kết nối tới máy chủ `192.168.1.24`, dữ liệu được lưu trữ an toàn trong thư mục `spool/pending_upload`. Ngay khi có mạng trở lại, uploader sẽ tự động truyền tiếp mà không bị mất dữ liệu.
- **Bảo mật tối đa:** Dữ liệu token và phiên làm việc được bảo vệ bằng cơ chế mã hóa Windows DPAPI phần cứng và truyền qua giao thức HTTPS có xác thực chứng chỉ số CA.

---

## 5. KIỂM TRA TRẠNG THÁI VÀ GIÁM SÁT

### Kiểm tra trên máy trạm:
Mở `Task Manager` (hoặc PowerShell) kiểm tra 2 tiến trình sau đang hoạt động:
```powershell
Get-Process trajectory-agent, trajectory-uploader
```

### Kiểm tra trên máy chủ (Server 192.168.1.24):
Dữ liệu nhịp tim (Heartbeat) và các phiên làm việc (Sessions) tự động ghi nhận vào cơ sở dữ liệu PostgreSQL và lưu trữ MinIO S3:
- Kiểm tra danh sách thiết bị đang kết nối:
```sql
SELECT machine_id, hostname, status, last_heartbeat_at 
FROM machines 
ORDER BY last_heartbeat_at DESC;
```

---

## 6. GỠ BỎ CÀI ĐẶT (KHI CẦN THIẾT)

Khi muốn gỡ bỏ hoàn toàn ứng dụng trên máy trạm:
1. Nhấp đúp vào tệp **`uninstall_autostart.bat`**.
2. Hệ thống sẽ tự động tắt các tiến trình đang chạy, xóa bỏ khóa Windows Registry và phím tắt Startup.
