<#
.SYNOPSIS
    Starts the Trajectory Ingestion Server stack via Docker Compose.
.DESCRIPTION
    Launches PostgreSQL, MinIO (HTTPS), Trajectory Server, and Caddy Reverse Proxy.
    Validates health endpoint after startup.
#>

[CmdletBinding()]
param()

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $ScriptDir "docker-compose.server.yml"
$EnvFile = Join-Path $ScriptDir "server.env"
$CaCert = Join-Path $ScriptDir "tls\ca.crt"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Starting Trajectory Ingestion Server (Docker Compose)   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Error "Docker command not found. Please ensure Docker Desktop is running."
    exit 1
}

if (-not (Test-Path "$EnvFile")) {
    Write-Error "Configuration file not found: $EnvFile"
    exit 1
}

Write-Host "[1/3] Bringing up containers..." -ForegroundColor Yellow
docker compose --env-file "$EnvFile" -p trajectory -f "$ComposeFile" up -d

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to start Docker Compose stack."
    exit $LASTEXITCODE
}

Write-Host "[2/3] Waiting for server health check..." -ForegroundColor Yellow
$maxRetries = 20
$healthy = $false
$healthUrl = "https://127.0.0.1/api/v1/health"

for ($i = 1; $i -le $maxRetries; $i++) {
    Start-Sleep -Seconds 1
    try {
        $res = & curl.exe --ssl-no-revoke --cacert "$CaCert" -s "$healthUrl"
        if ($res -match '"status":\s*"healthy"') {
            $healthy = $true
            break
        }
    } catch {
        # Retry
    }
}

if ($healthy) {
    Write-Host "[3/3] Server is HEALTHY and READY!" -ForegroundColor Green
} else {
    Write-Host "[WARNING] Health check timed out or not yet ready. Checking docker status..." -ForegroundColor Red
}

Write-Host ""
Write-Host "================== SERVER ACCESS POINTS ==================" -ForegroundColor Cyan
Write-Host "  * Host IP (LAN):       192.168.1.24" -ForegroundColor White
Write-Host "  * Ingestion API URL:   https://192.168.1.24" -ForegroundColor White
Write-Host "  * Web Dashboard:       https://192.168.1.24/dashboard/" -ForegroundColor White
Write-Host "  * Dashboard Token:     trajectory-admin-dashboard-api-token-32-chars-2026" -ForegroundColor Gray
Write-Host "  * MinIO HTTPS Console: https://192.168.1.24:9001" -ForegroundColor White
Write-Host "  * MinIO Admin User:    trajectory-minio-admin" -ForegroundColor Gray
Write-Host "  * PostgreSQL:          localhost:5432 (DB: trajectory)" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Cyan
