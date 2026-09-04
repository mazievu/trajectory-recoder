[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$ConfigPath,

    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Container })]
    [string]$InstallDirectory,

    [Parameter(Mandatory = $true)]
    [string]$UserId,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-p]{32}$')]
    [string]$ChromeExtensionId,

    [ValidatePattern('^[a-p]{32}$')]
    [string]$EdgeExtensionId,

    [string]$TaskName = 'Trajectory Recorder Interactive Capture',

    # Override points keep integration tests isolated from a user's production
    # task and native-messaging registration. The defaults are the production
    # locations documented below.
    [ValidatePattern('^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$')]
    [string]$NativeHostName = 'com.trajectory.recorder.browser_host',

    [ValidateNotNullOrEmpty()]
    [string]$ManifestDirectory = (Join-Path $env:LOCALAPPDATA 'TrajectoryRecorder\native-messaging'),

    [switch]$Remove
)

$ErrorActionPreference = 'Stop'

$agentPath = Join-Path $InstallDirectory 'trajectory-agent.exe'
$browserHostPath = Join-Path $InstallDirectory 'trajectory-browser-host.exe'
$manifestPath = Join-Path $ManifestDirectory "$NativeHostName.json"

if ($Remove) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$NativeHostName" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$NativeHostName" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
    Write-Host "Removed interactive capture task, per-user native messaging registrations, and manifest."
    exit 0
}

if (-not (Test-Path -LiteralPath $agentPath -PathType Leaf)) {
    throw "Missing interactive capture executable: $agentPath"
}
if (-not (Test-Path -LiteralPath $browserHostPath -PathType Leaf)) {
    throw "Missing browser native host executable: $browserHostPath"
}

# The config is parsed before task creation so a server or malformed config can
# never be scheduled in a user's interactive desktop session.
& (Join-Path $PSScriptRoot 'Validate-RoleConfiguration.ps1') -ConfigPath $ConfigPath -ExpectedRole client
if ($LASTEXITCODE -ne 0) {
    throw 'Client configuration validation failed.'
}

$origins = @("chrome-extension://$ChromeExtensionId/")
if (-not [string]::IsNullOrWhiteSpace($EdgeExtensionId)) {
    $origins += "chrome-extension://$EdgeExtensionId/"
}

$manifest = @{
    name = $NativeHostName
    description = 'Trajectory Recorder Native Messaging Host Bridge'
    path = $browserHostPath
    type = 'stdio'
    allowed_origins = $origins
} | ConvertTo-Json -Depth 3

if ($PSCmdlet.ShouldProcess($manifestPath, 'write per-user native messaging manifest')) {
    New-Item -ItemType Directory -Path $ManifestDirectory -Force | Out-Null
    Set-Content -LiteralPath $manifestPath -Value $manifest -Encoding utf8 -NoNewline

    $chromeKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$NativeHostName"
    New-Item -Path $chromeKey -Force | Out-Null
    Set-Item -Path $chromeKey -Value $manifestPath
    if (-not [string]::IsNullOrWhiteSpace($EdgeExtensionId)) {
        $edgeKey = "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$NativeHostName"
        New-Item -Path $edgeKey -Force | Out-Null
        Set-Item -Path $edgeKey -Value $manifestPath
    }
}

# InteractiveToken guarantees the agent runs only in this user's logon session.
# Browser-host is deliberately not a scheduled process: Chromium/Edge starts it
# over stdio on demand through the above HKCU native-messaging registration.
$action = New-ScheduledTaskAction -Execute $agentPath -Argument "--config `"$ConfigPath`""
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $UserId
# The Task Scheduler XML value is InteractiveToken. Windows PowerShell exposes
# that schema value through the ScheduledTasks cmdlet as the `Interactive` enum.
$principal = New-ScheduledTaskPrincipal -UserId $UserId -LogonType Interactive -RunLevel Limited
if ($PSCmdlet.ShouldProcess($TaskName, "register interactive logon task for $UserId")) {
    Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Force | Out-Null
}

Write-Host "Installed interactive capture for $UserId. Browser-host will be started by Chrome/Edge in the user's session, not by Session 0."
