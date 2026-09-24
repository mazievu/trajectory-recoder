<#
.SYNOPSIS
    Script thu thập phần cứng cục bộ chạy ngầm (Silent), không làm gián đoạn người dùng.
.DESCRIPTION
    Đọc trực tiếp WMI/CIM cục bộ của máy tính hiện tại, lấy thông tin CPU, RAM, GPU, Ổ đĩa
    và tự động gửi/ghi về máy chủ trung tâm 192.168.1.24.
#>

$ErrorActionPreference = "SilentlyContinue"

try {
    $cs  = Get-CimInstance Win32_ComputerSystem
    $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
    $os  = Get-CimInstance Win32_OperatingSystem
    
    # GPU
    $gpuList = (Get-CimInstance Win32_VideoController | Where-Object { 
        $_.Name -notlike "*Virtual*" -and $_.Name -notlike "*Basic*" 
    } | Select-Object -ExpandProperty Name) -join " + "
    if (-not $gpuList) {
        $gpuList = (Get-CimInstance Win32_VideoController | Select-Object -First 1).Name
    }

    # RAM
    $ramTotalGB = [math]::Round($cs.TotalPhysicalMemory / 1GB, 1)
    $ramFreeGB  = [math]::Round($os.FreePhysicalMemory / 1MB, 1)

    # Disks
    $diskInfo = (Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3" | ForEach-Object {
        $free = [math]::Round($_.FreeSpace / 1GB, 1)
        $total = [math]::Round($_.Size / 1GB, 1)
        $pct = [math]::Round(($_.Size - $_.FreeSpace) / $_.Size * 100, 0)
        "$($_.DeviceID) $free/$($total)GB ($pct%)"
    }) -join " | "

    # Local IP Address
    $localIP = (Get-NetIPAddress -AddressFamily IPv4 -InterfaceAlias "Ethernet*", "Wi-Fi*" -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -like "192.168.*" } | Select-Object -First 1).IPAddress
    if (-not $localIP) { $localIP = "127.0.0.1" }

    $data = [PSCustomObject]@{
        Timestamp    = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
        MachineName  = $env:COMPUTERNAME
        UserName     = $env:USERNAME
        IPAddress    = $localIP
        CPU          = $cpu.Name
        Cores        = $cpu.NumberOfCores
        Threads      = $cpu.NumberOfLogicalProcessors
        RAM_Total_GB = $ramTotalGB
        RAM_Free_GB  = $ramFreeGB
        GPU          = $gpuList
        Disks        = $diskInfo
        OS_Version   = $os.Caption
    }

    # 1. Thử gửi về HTTP Server trung tâm nếu có listener
    try {
        $json = $data | ConvertTo-Json -Compress
        $response = Invoke-RestMethod -Uri "http://192.168.1.24:9999/api/hardware" -Method Post -Body $json -ContentType "application/json" -TimeoutSec 2 -ErrorAction SilentlyContinue
    } catch {}

    # 2. Thử ghi vào thư mục SMB dùng chung trên máy chủ
    $sharePath = "\\192.168.1.24\OpenClaw-Knowledge\hardware_inventory.csv"
    try {
        if (-not (Test-Path $sharePath)) {
            "Timestamp,MachineName,UserName,IPAddress,CPU,Cores,Threads,RAM_Total_GB,RAM_Free_GB,GPU,Disks,OS_Version" | Out-File -FilePath $sharePath -Encoding utf8
        }
        
        $line = '"{0}","{1}","{2}","{3}","{4}",{5},{6},{7},{8},"{9}","{10}","{11}"' -f `
            $data.Timestamp, $data.MachineName, $data.UserName, $data.IPAddress, `
            $data.CPU.Replace('"', '""'), $data.Cores, $data.Threads, `
            $data.RAM_Total_GB, $data.RAM_Free_GB, $data.GPU.Replace('"', '""'), `
            $data.Disks, $data.OS_Version
            
        $line | Out-File -FilePath $sharePath -Append -Encoding utf8
    } catch {}

    # 3. Luôn lưu 1 bản snapshot cục bộ
    $localSnapDir = "C:\ProgramData\TrajectoryHardware"
    if (-not (Test-Path $localSnapDir)) { New-Item -ItemType Directory -Path $localSnapDir -Force | Out-Null }
    $data | Export-Clixml -Path "$localSnapDir\hardware.xml" -Force
} catch {}
