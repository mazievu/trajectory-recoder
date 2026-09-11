<#
.SYNOPSIS
    Stops the Trajectory Ingestion Server stack.
.DESCRIPTION
    Stops and removes Docker Compose containers for the trajectory project gracefully.
#>

[CmdletBinding()]
param()

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $ScriptDir "docker-compose.server.yml"
$EnvFile = Join-Path $ScriptDir "server.env"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Stopping Trajectory Ingestion Server (Docker Compose)   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

docker compose --env-file "$EnvFile" -p trajectory -f "$ComposeFile" down

if ($LASTEXITCODE -eq 0) {
    Write-Host "Trajectory server stack has been safely stopped." -ForegroundColor Green
} else {
    Write-Error "Failed to stop Docker Compose stack cleanly."
    exit $LASTEXITCODE
}
