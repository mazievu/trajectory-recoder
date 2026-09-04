$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$composePath = Join-Path $PSScriptRoot 'docker-compose.full-stack-e2e.yml'
$runnerPath = Join-Path $PSScriptRoot 'Run-FullStackDockerE2E.ps1'

if (-not (Test-Path -LiteralPath $composePath)) {
    throw 'Missing production-like full-stack Compose fixture.'
}

if (-not (Test-Path -LiteralPath $runnerPath)) {
    throw 'Missing deterministic full-stack Docker E2E runner.'
}

$compose = Get-Content -LiteralPath $composePath -Raw
foreach ($requiredService in @('postgres:', 'minio:', 'minio-init:', 'server:', 'proxy:')) {
    if (-not $compose.Contains($requiredService)) {
        throw "E2E Compose fixture must define service '$requiredService'."
    }
}

foreach ($requiredContract in @(
    'S3_ENDPOINT: https://minio:9000',
    'S3_CA_CERT_PATH: /run/trajectory-e2e-certs/ca.crt',
    'condition: service_completed_successfully',
    '127.0.0.1:${E2E_PROXY_PORT:-8443}:443'
)) {
    if (-not $compose.Contains($requiredContract)) {
        throw "E2E Compose fixture is missing contract '$requiredContract'."
    }
}

$runner = Get-Content -LiteralPath $runnerPath -Raw
foreach ($requiredVerification in @(
    '/api/v1/health',
    '/dashboard/',
    '/api/v1/machines/register',
    '/api/v1/sessions/',
    'mc stat',
    'sqlx',
    'ca.crt'
)) {
    if (-not $runner.Contains($requiredVerification)) {
        throw "E2E runner is missing verification '$requiredVerification'."
    }
}

$productionEntrypoint = Join-Path $repositoryRoot 'server/docker-entrypoint.sh'
if (-not (Test-Path -LiteralPath $productionEntrypoint)) {
    throw 'Missing server entrypoint for operator-provided private S3 CA certificates.'
}
$entrypoint = Get-Content -LiteralPath $productionEntrypoint -Raw
foreach ($requiredEntrypointContract in @('S3_CA_CERT_PATH', 'update-ca-certificates', 'exec /usr/local/bin/trajectory-server')) {
    if (-not $entrypoint.Contains($requiredEntrypointContract)) {
        throw "Server entrypoint is missing S3 CA trust contract '$requiredEntrypointContract'."
    }
}

Write-Host 'Full-stack Docker E2E contract tests passed.'
