$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$installer = Join-Path $repositoryRoot 'deployment\Install-InteractiveCaptureTask.ps1'
$nativeHostName = "com.trajectory.recorder.test.$([guid]::NewGuid().ToString('N'))"
$taskName = "Trajectory Recorder Test $([guid]::NewGuid().ToString('N'))"
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) "trajectory-recorder-installer-test-$([guid]::NewGuid().ToString('N'))"
$manifestDirectory = Join-Path $fixtureRoot 'native-messaging'
$installDirectory = Join-Path $fixtureRoot 'bin'
$configPath = Join-Path $fixtureRoot 'client.env'
$agentPath = Join-Path $installDirectory 'trajectory-agent.exe'
$browserHostPath = Join-Path $installDirectory 'trajectory-browser-host.exe'
$chromeKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$nativeHostName"
$edgeKey = "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$nativeHostName"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Remove-TestArtifacts {
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $chromeKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $edgeKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}

try {
    New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
    New-Item -ItemType File -Path $agentPath -Force | Out-Null
    New-Item -ItemType File -Path $browserHostPath -Force | Out-Null
    @(
        'DEPLOYMENT_ROLE=client'
        'TRAJECTORY_SERVER_URL=https://collector.example.test'
        'TRAJECTORY_MACHINE_ID=INSTALLER-TEST-MACHINE'
        'TRAJECTORY_USER_ID=installer-test-user'
        'TRAJECTORY_ENROLLMENT_TOKEN=test-enrollment-token'
        "SPOOL_DIR=$fixtureRoot\spool"
    ) | Set-Content -LiteralPath $configPath -Encoding utf8

    $currentUser = (& whoami).Trim()
    $chromeExtensionId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
    $edgeExtensionId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
    $arguments = @{
        ConfigPath = $configPath
        InstallDirectory = $installDirectory
        UserId = $currentUser
        ChromeExtensionId = $chromeExtensionId
        EdgeExtensionId = $edgeExtensionId
        TaskName = $taskName
        NativeHostName = $nativeHostName
        ManifestDirectory = $manifestDirectory
    }

    & $installer @arguments
    if ($LASTEXITCODE -ne 0) { throw "Installer failed with exit code $LASTEXITCODE" }

    $task = Get-ScheduledTask -TaskName $taskName -ErrorAction Stop
    Assert-True ($task.Principal.LogonType -eq 'InteractiveToken') 'Task must use InteractiveToken.'
    Assert-True ($task.Principal.UserId -eq $currentUser) 'Task must target the requested signed-in user.'
    Assert-True ($task.Actions[0].Execute -eq $agentPath) 'Task must launch the fixture capture agent.'
    Assert-True ($task.Actions[0].Arguments -match [regex]::Escape($configPath)) 'Task must pass the explicit fixture config.'

    $manifestPath = Join-Path $manifestDirectory "$nativeHostName.json"
    Assert-True (Test-Path -LiteralPath $manifestPath -PathType Leaf) 'Installer must write the native-messaging manifest.'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    Assert-True ($manifest.path -eq $browserHostPath) 'Manifest must reference the fixture browser host.'
    Assert-True (@($manifest.allowed_origins).Count -eq 2) 'Manifest must contain both extension origins.'
    Assert-True (@($manifest.allowed_origins) -contains "chrome-extension://$chromeExtensionId/") 'Manifest must allow Chrome extension origin.'
    Assert-True (@($manifest.allowed_origins) -contains "chrome-extension://$edgeExtensionId/") 'Manifest must allow Edge extension origin.'
    Assert-True ((Get-ItemPropertyValue -LiteralPath $chromeKey -Name '(default)') -eq $manifestPath) 'Chrome registration must point to the manifest.'
    Assert-True ((Get-ItemPropertyValue -LiteralPath $edgeKey -Name '(default)') -eq $manifestPath) 'Edge registration must point to the manifest.'

    & $installer @arguments -Remove
    if ($LASTEXITCODE -ne 0) { throw "Installer removal failed with exit code $LASTEXITCODE" }
    Assert-True (-not (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue)) 'Removal must unregister the scoped task.'
    Assert-True (-not (Test-Path -LiteralPath $chromeKey)) 'Removal must delete the scoped Chrome registration.'
    Assert-True (-not (Test-Path -LiteralPath $edgeKey)) 'Removal must delete the scoped Edge registration.'
    Assert-True (-not (Test-Path -LiteralPath $manifestPath)) 'Removal must delete the native-messaging manifest.'

    Write-Host 'Interactive capture installer integration test passed.'
}
finally {
    Remove-TestArtifacts
}
