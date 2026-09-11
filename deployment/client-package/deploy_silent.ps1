<#
.SYNOPSIS
    Silent enterprise deployment script for Trajectory Recorder client.
.DESCRIPTION
    Installs and registers Trajectory Recorder to run completely silently in the background
    for all machine users or current user, with zero interactive prompts.
    Ideal for GPO, SCCM, Intune, or remote PowerShell mass deployments.
.PARAMETER TargetDir
    Target installation directory. Default is current script directory.
.PARAMETER ServerUrl
    Trajectory Ingestion Server URL. Default: https://192.168.1.24
.PARAMETER MachineId
    Machine identifier. Default: $env:COMPUTERNAME
.PARAMETER UserId
    User identifier. Default: $env:USERNAME
.PARAMETER EnrollmentToken
    Shared enrollment token. Default: trajectory-client-enrollment-token-2026
.EXAMPLE
    .\deploy_silent.ps1
.EXAMPLE
    .\deploy_silent.ps1 -MachineId "IT-DEPT-01" -UserId "alice"
#>

[CmdletBinding()]
param (
    [string]$TargetDir = $PSScriptRoot,
    [string]$ServerUrl = "https://192.168.1.24",
    [string]$MachineId = $env:COMPUTERNAME,
    [string]$UserId = $env:USERNAME,
    [string]$EnrollmentToken = "trajectory-client-enrollment-token-2026"
)

$ErrorActionPreference = "Stop"

if (-not $MachineId) { $MachineId = "PC-$((Get-Random) % 90000 + 10000)" }
if (-not $UserId) { $UserId = "employee" }

Write-Output "[DEPLOY] Starting silent deployment for Trajectory Recorder..."
Write-Output "[DEPLOY] TargetDir: $TargetDir"
Write-Output "[DEPLOY] MachineId: $MachineId"
Write-Output "[DEPLOY] UserId: $UserId"
Write-Output "[DEPLOY] ServerUrl: $ServerUrl"

# 1. Stop old processes if running
Get-Process -Name "trajectory-agent", "trajectory-uploader" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# 2. Ensure spool directories
$spoolDir = Join-Path $TargetDir "spool"
$subDirs = @("recording", "finalizing", "pending_upload", "uploading", "uploaded", "failed")
foreach ($sub in $subDirs) {
    $path = Join-Path $spoolDir $sub
    if (-not (Test-Path $path)) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
    }
}

# 3. Write client.env
$envFile = Join-Path $TargetDir "client.env"
$envContent = @"
DEPLOYMENT_ROLE=client
TRAJECTORY_SERVER_URL=$ServerUrl
TRAJECTORY_MACHINE_ID=$MachineId
TRAJECTORY_USER_ID=$UserId
SPOOL_DIR=$spoolDir
TRAJECTORY_ENROLLMENT_TOKEN=$EnrollmentToken
"@
[System.IO.File]::WriteAllText($envFile, $envContent, [System.Text.Encoding]::ASCII)

# 4. Register Autostart (HKCU Registry Run + Startup Folder)
$vbsLauncher = Join-Path $TargetDir "start_silent.vbs"

# Registry Run
$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $regPath -Name "TrajectoryRecorder" -Value "wscript.exe `"$vbsLauncher`"" -Force

# Startup shortcut
$startupDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Startup)
if ($startupDir -and (Test-Path $startupDir)) {
    $wsh = New-Object -ComObject WScript.Shell
    $shortcut = $wsh.CreateShortcut((Join-Path $startupDir "TrajectoryRecorder.lnk"))
    $shortcut.TargetPath = "wscript.exe"
    $shortcut.Arguments = "`"$vbsLauncher`""
    $shortcut.WorkingDirectory = $TargetDir
    $shortcut.WindowStyle = 7
    $shortcut.Description = "Trajectory Recorder Background Service"
    $shortcut.Save()
}

# 5. Launch hidden processes
Start-Process -FilePath "wscript.exe" -ArgumentList "`"$vbsLauncher`"" -WorkingDirectory $TargetDir -WindowStyle Hidden

# 6. Verify running state
Start-Sleep -Seconds 3
$agentRunning = Get-Process -Name "trajectory-agent" -ErrorAction SilentlyContinue
$uploaderRunning = Get-Process -Name "trajectory-uploader" -ErrorAction SilentlyContinue

$status = @{
    Timestamp = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    MachineId = $MachineId
    UserId = $UserId
    AgentRunning = ($null -ne $agentRunning)
    UploaderRunning = ($null -ne $uploaderRunning)
    Status = if ($agentRunning -and $uploaderRunning) { "SUCCESS" } else { "PARTIAL" }
}

Write-Output ($status | ConvertTo-Json -Compress)
