<#
.SYNOPSIS
    Quét và thu thập thông tin phần cứng (CPU, RAM, GPU, Disk) của 12 máy trạm qua mạng LAN.
.DESCRIPTION
    Script chạy từ máy chủ (192.168.1.24), tự động kiểm tra kết nối mạng (Ping, Port 135/445/5985),
    và truy vấn WMI/CIM ngầm từ xa mà không làm gián đoạn hay ảnh hưởng tới màn hình của nhân viên.
.PARAMETER Credential
    Tài khoản quản trị (nếu các máy trạm có mật khẩu chung). Bỏ trống nếu thử dùng quyền hiện tại.
.EXAMPLE
    .\Scan-LanHardware.ps1
.EXAMPLE
    .\Scan-LanHardware.ps1 -Credential (Get-Credential)
#>

[CmdletBinding()]
param (
    [System.Management.Automation.PSCredential]$Credential = $null,
    [string]$OutputFile = "D:\tools GTF\trajectory recoder\report\HARDWARE_AUDIT_LAN.md"
)

$KnownMachines = @(
    @{ Name = "NGOCMINH-PC";     Role = "Management / Server Workstation"; KnownUser = "admin" },
    @{ Name = "MR-QUAN";         Role = "Lead Video Producer";             KnownUser = "PC" },
    @{ Name = "EDIT-KIEN-2";     Role = "Senior Video Editor";             KnownUser = "giftt" },
    @{ Name = "EDIT-KIEN-3";     Role = "Secondary Render Workstation";    KnownUser = "Admin" },
    @{ Name = "EDIT-NGOCMINH";   Role = "Tech Lead / AI Developer";        KnownUser = "gifft" },
    @{ Name = "DESKTOP-1Q714D7"; Role = "Graphic & Vector Designer";       KnownUser = "admin" },
    @{ Name = "DESKTOP-3F6NKQA"; Role = "Video Editor (Nail Art)";         KnownUser = "giftt" },
    @{ Name = "DESKTOP-K5AMUHI"; Role = "Media Buyer (TikTok Ads)";        KnownUser = "admin" },
    @{ Name = "DESKTOP-MCV5QUC"; Role = "Content Strategist";              KnownUser = "giftt" },
    @{ Name = "DESKTOP-VO7GFI0"; Role = "Video Creator (Baby Carrier)";    KnownUser = "admin" },
    @{ Name = "GTF-22";          Role = "Marketing & Ads Ops";             KnownUser = "gifft" },
    @{ Name = "TESTER-PC";       Role = "QA Testing Station";              KnownUser = "tester" }
)

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host " GTF LAN HARDWARE SCANNER - THU THẬP CẤU HÌNH CPU & RAM TỪ XA" -ForegroundColor Yellow
Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "[*] Bắt đầu quét $($KnownMachines.Count) máy trạm trong danh sách công ty..." -ForegroundColor Gray

$results = @()

foreach ($item in $KnownMachines) {
    $name = $item.Name
    $role = $item.Role
    $u = $item.KnownUser
    
    Write-Host "`n[*] Đang kiểm tra: $name ($role)..." -NoNewline
    
    $obj = [PSCustomObject]@{
        MachineName   = $name
        Role          = $role
        KnownUser     = $u
        IPAddress     = "Unknown"
        PingStatus    = "Offline"
        SmbPort445    = "Closed"
        RpcPort135    = "Closed"
        CPU           = "Chưa kết nối WMI"
        CoresThreads  = "N/A"
        RAM_Total_GB  = "N/A"
        RAM_Free_GB   = "N/A"
        GPU           = "N/A"
        Disks         = "N/A"
        OS_Version    = "N/A"
        ScanMethod    = "None"
    }

    # 1. Nếu là máy cục bộ hiện tại (Local)
    if ($name -eq $env:COMPUTERNAME -or $name -eq "NGOCMINH-PC") {
        Write-Host " [MÁY LOCAL HIỆN TẠI]" -ForegroundColor Green
        $obj.IPAddress = "127.0.0.1 (192.168.1.24)"
        $obj.PingStatus = "Online"
        $obj.SmbPort445 = "Open"
        $obj.RpcPort135 = "Open"
        $obj.ScanMethod = "Local WMI"
        
        try {
            $cpuInfo = Get-CimInstance Win32_Processor | Select-Object -First 1
            $obj.CPU = $cpuInfo.Name
            $obj.CoresThreads = "$($cpuInfo.NumberOfCores) Cores / $($cpuInfo.NumberOfLogicalProcessors) Threads"
            
            $cs = Get-CimInstance Win32_ComputerSystem
            $os = Get-CimInstance Win32_OperatingSystem
            $obj.RAM_Total_GB = [math]::Round($cs.TotalPhysicalMemory / 1GB, 1)
            $obj.RAM_Free_GB  = [math]::Round($os.FreePhysicalMemory / 1MB, 1)
            $obj.OS_Version   = $os.Caption
            
            $gpus = (Get-CimInstance Win32_VideoController | Where-Object { $_.Name -notlike "*Virtual*" -and $_.Name -notlike "*Basic*" } | Select-Object -ExpandProperty Name) -join ", "
            if ($gpus) {
                $obj.GPU = $gpus
            } else {
                $obj.GPU = (Get-CimInstance Win32_VideoController | Select-Object -First 1).Name
            }
            
            $diskList = @()
            $allDrives = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3"
            foreach ($d in $allDrives) {
                $freeG = [math]::Round($d.FreeSpace / 1GB, 1)
                $totG = [math]::Round($d.Size / 1GB, 1)
                $diskList += "$($d.DeviceID) ${freeG}GB/${totG}GB"
            }
            $obj.Disks = $diskList -join " | "
        } catch {
            $obj.CPU = "Lỗi đọc: $_"
        }
        $results += $obj
        continue
    }

    # 2. Phân giải DNS / IP
    $dns = Resolve-DnsName -Name $name -ErrorAction SilentlyContinue
    $targetIP = $null
    if ($dns) {
        $ipv4 = $dns | Where-Object { $_.IPAddress -like "192.168.*" } | Select-Object -First 1
        if ($ipv4) {
            $targetIP = $ipv4.IPAddress
        } else {
            $targetIP = $dns[0].IPAddress
        }
    }

    if ($targetIP) {
        $obj.IPAddress = $targetIP
    }

    # 3. Test Ping (1 packet, timeout 800ms)
    $pingTarget = $name
    if ($targetIP) { $pingTarget = $targetIP }
    $ping = $false
    try {
        $pinger = New-Object System.Net.NetworkInformation.Ping
        $reply = $pinger.Send($pingTarget, 800)
        if ($reply.Status -eq [System.Net.NetworkInformation.IPStatus]::Success) { $ping = $true }
    } catch {}
    if ($ping) {
        $obj.PingStatus = "Online (Ping OK)"
        Write-Host " [ONLINE]" -ForegroundColor Green -NoNewline
    } else {
        $obj.PingStatus = "No Ping Response"
        Write-Host " [NO PING]" -ForegroundColor DarkYellow -NoNewline
    }

    # 4. Kiểm tra cổng mạng RPC (135) và SMB (445)
    if ($targetIP) {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $connect = $tcpClient.BeginConnect($targetIP, 135, $null, $null)
        $wait = $connect.AsyncWaitHandle.WaitOne(600, $false)
        if ($wait -and $tcpClient.Connected) {
            $obj.RpcPort135 = "Open"
            $tcpClient.EndConnect($connect)
        }
        $tcpClient.Close()

        $tcpClient2 = New-Object System.Net.Sockets.TcpClient
        $connect2 = $tcpClient2.BeginConnect($targetIP, 445, $null, $null)
        $wait2 = $connect2.AsyncWaitHandle.WaitOne(600, $false)
        if ($wait2 -and $tcpClient2.Connected) {
            $obj.SmbPort445 = "Open"
            $tcpClient2.EndConnect($connect2)
        }
        $tcpClient2.Close()
    }

    # 5. Thử truy vấn WMI ngầm từ xa
    $wmiSuccess = $false
    $targetConnect = $name
    if ($targetIP) { $targetConnect = $targetIP }
    
    $cimParams = @{
        ComputerName        = $targetConnect
        OperationTimeoutSec = 3
        ErrorAction         = "Stop"
    }
    if ($Credential) {
        $cimParams["Credential"] = $Credential
    }

    try {
        $remCpu = Get-CimInstance Win32_Processor @cimParams | Select-Object -First 1
        if ($remCpu) {
            $obj.CPU = $remCpu.Name
            $obj.CoresThreads = "$($remCpu.NumberOfCores) Cores / $($remCpu.NumberOfLogicalProcessors) Threads"
            
            $remCs = Get-CimInstance Win32_ComputerSystem @cimParams
            $remOs = Get-CimInstance Win32_OperatingSystem @cimParams
            $obj.RAM_Total_GB = [math]::Round($remCs.TotalPhysicalMemory / 1GB, 1)
            $obj.RAM_Free_GB  = [math]::Round($remOs.FreePhysicalMemory / 1MB, 1)
            $obj.OS_Version   = $remOs.Caption

            $remGpu = (Get-CimInstance Win32_VideoController @cimParams | Where-Object { $_.Name -notlike "*Virtual*" -and $_.Name -notlike "*Basic*" } | Select-Object -ExpandProperty Name) -join ", "
            if ($remGpu) {
                $obj.GPU = $remGpu
            } else {
                $obj.GPU = (Get-CimInstance Win32_VideoController @cimParams | Select-Object -First 1).Name
            }

            $remDiskList = @()
            $remDrives = Get-CimInstance Win32_LogicalDisk @cimParams -Filter "DriveType=3"
            foreach ($d in $remDrives) {
                $freeG = [math]::Round($d.FreeSpace / 1GB, 1)
                $totG = [math]::Round($d.Size / 1GB, 1)
                $remDiskList += "$($d.DeviceID) ${freeG}GB/${totG}GB"
            }
            $obj.Disks = $remDiskList -join " | "
            $obj.ScanMethod = "Remote CIM/WMI"
            $wmiSuccess = $true
            Write-Host " -> WMI THÀNH CÔNG!" -ForegroundColor Green
        }
    } catch {
        $msg = $_.Exception.Message
        if ($msg -like "*Access is denied*" -or $msg -like "*bị từ chối*") {
            $obj.CPU = "Cần quyền Admin (Access Denied)"
            Write-Host " -> Cần mật khẩu Admin LAN" -ForegroundColor Yellow
        } else {
            $shortMsg = $msg.Split("`n")[0].Trim()
            if ($shortMsg.Length -gt 60) { $shortMsg = $shortMsg.Substring(0, 60) + "..." }
            $obj.CPU = "Lỗi: $shortMsg"
            Write-Host " -> Chưa mở WMI" -ForegroundColor DarkGray
        }
    }

    $results += $obj
}

# Hiển thị bảng tổng kết trên console
Write-Host "`n=================================================================" -ForegroundColor Cyan
Write-Host " BẢNG KẾT QUẢ THU THẬP CẤU HÌNH PHẦN CỨNG QUA LAN" -ForegroundColor Yellow
Write-Host "=================================================================" -ForegroundColor Cyan

$results | Format-Table -Property MachineName, IPAddress, PingStatus, CPU, RAM_Total_GB, GPU -AutoSize

# Xuất ra Markdown báo cáo
$md = @()
$md += "# BÁO CÁO CẤU HÌNH PHẦN CỨNG DÀN MÁY TRẠM (GTF LAN)"
$md += "**Thời điểm quét**: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  "
$md += "**Máy chủ quét**: $env:COMPUTERNAME (192.168.1.24)  "
$md += ""
$md += "---"
$md += ""
$md += "## 1. BẢNG CHI TIẾT CẤU HÌNH PHẦN CỨNG"
$md += ""
$md += "| Máy trạm | IP LAN | Trạng thái | Vi xử lý (CPU) | Tổng RAM (GB) | Card Đồ họa (GPU) | Dung lượng Ổ đĩa | Phương thức |"
$md += "| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

foreach ($r in $results) {
    $md += "| **$($r.MachineName)** | $($r.IPAddress) | $($r.PingStatus) | $($r.CPU) | **$($r.RAM_Total_GB)** | $($r.GPU) | $($r.Disks) | $($r.ScanMethod) |"
}

$md += ""
$md += "---"
$md += ""
$md += "## 2. HƯỚNG DẪN BỔ SUNG NẾU MÁY BÁO 'ACCESS DENIED'"
$md += "Trong mạng Windows Workgroup (không gia nhập Domain Active Directory), Windows mặc định chặn truy vấn WMI từ máy khác nếu không cung cấp User/Password có quyền Administrator trên máy đó."
$md += ""
$md += '1. **Nếu bạn có mật khẩu Admin chung**: Chạy lệnh kèm tham số `-Credential`:'
$md += '   `.\Scan-LanHardware.ps1 -Credential (Get-Credential)`'
$md += '2. **Giải pháp 100% không cần mật khẩu**: Gửi tệp `Collect-Silent.vbs` cho nhân viên nhấp đúp (hoặc chạy qua USB/Lark). Script sẽ tự chạy ngầm 0.3s không hiện cửa sổ và gửi thông số về máy chủ.'

$mdContent = $md -join "`r`n"
$outputDir = Split-Path $OutputFile -Parent
if (-not (Test-Path $outputDir)) { New-Item -ItemType Directory -Path $outputDir -Force | Out-Null }
[System.IO.File]::WriteAllText($OutputFile, $mdContent, [System.Text.Encoding]::UTF8)

Write-Host "`n[+] Đã lưu báo cáo markdown tại: $OutputFile" -ForegroundColor Green
return $results
