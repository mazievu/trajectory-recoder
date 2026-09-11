<#
.SYNOPSIS
    Checks the status of the Trajectory Ingestion Server stack.
#>

[CmdletBinding()]
param()

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $ScriptDir "docker-compose.server.yml"
$EnvFile = Join-Path $ScriptDir "server.env"
$CaCert = Join-Path $ScriptDir "tls\ca.crt"
$Token = "trajectory-admin-dashboard-api-token-32-chars-2026"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "            Trajectory Server Stack Status                " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "[Container State]" -ForegroundColor Yellow
docker compose --env-file "$EnvFile" -p trajectory -f "$ComposeFile" ps

Write-Host "`n[Health Endpoint Probe]" -ForegroundColor Yellow
try {
    $healthRes = & curl.exe --ssl-no-revoke --cacert "$CaCert" -s https://127.0.0.1/api/v1/health
    Write-Host "Health Check: $healthRes" -ForegroundColor Green
} catch {
    Write-Host "Health Check failed: $_" -ForegroundColor Red
}

Write-Host "`n[Registered Machines]" -ForegroundColor Yellow
try {
    $machinesRes = & curl.exe --ssl-no-revoke --cacert "$CaCert" -s -H "X-Server-Token: $Token" https://127.0.0.1/api/v1/machines
    Write-Host "Machines: $machinesRes" -ForegroundColor Green
} catch {
    Write-Host "Failed to query machines: $_" -ForegroundColor Red
}
