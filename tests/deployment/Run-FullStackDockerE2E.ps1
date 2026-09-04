[CmdletBinding()]
param(
    [switch]$KeepArtifacts
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$composePath = Join-Path $PSScriptRoot 'docker-compose.full-stack-e2e.yml'
$fixturePath = Join-Path $PSScriptRoot 'fixtures/full-stack-e2e.config'
$artifactRoot = Join-Path $PSScriptRoot '.artifacts/full-stack-docker-e2e'
$runId = "run-$PID"
$runDirectory = Join-Path $artifactRoot $runId
$certificateDirectory = Join-Path $runDirectory 'certificates'
$projectName = "trajectory-e2e-$PID"
$proxyPort = if ($env:E2E_PROXY_PORT) { $env:E2E_PROXY_PORT } else { '8443' }
$publicHostname = 'trajectory-e2e.test'
$dashboardPassword = 'trajectory-e2e-dashboard-password-32-bytes'
$enrollmentToken = 'trajectory-e2e-enrollment-token'
$sessionId = 'E2E_SESSION_0001'
$machineId = 'E2E_MACHINE_0001'
$payload = 'trajectory-e2e-payload'

function Invoke-Checked {
    param(
        [string]$Description,
        [scriptblock]$Action
    )

    $result = & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE."
    }
    return $result
}

function Invoke-Compose {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    Invoke-Checked -Description "docker compose $($Arguments -join ' ')" -Action {
        & docker compose --project-name $projectName --env-file $fixturePath -f $composePath @Arguments
    }
}

function Invoke-E2eCurl {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    Invoke-Checked -Description "HTTPS request to $publicHostname" -Action {
        & curl.exe --silent --show-error --fail --noproxy '*' --cacert (Join-Path $certificateDirectory 'ca.crt') `
            --resolve "${publicHostname}:${proxyPort}:127.0.0.1" @Arguments
    }
}

function ConvertTo-Sha256Hex {
    param([string]$Text)

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha256.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
    }
}

function New-TestCertificate {
    param([string]$OutputDirectory)

    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
    # Use a disposable container rather than host OpenSSL or a developer cert
    # store. Rustls validates a conventional CA -> leaf chain, not a leaf that
    # is also used as its own trust anchor.
    Invoke-Checked -Description 'test certificate generation' -Action {
        & docker run --rm --volume "${OutputDirectory}:/out" alpine:3.20 sh -ec `
            'apk add --no-cache openssl >/dev/null && openssl req -x509 -newkey rsa:2048 -nodes -keyout /out/ca.key -out /out/ca.crt -days 1 -subj "/CN=Trajectory E2E Test CA" -addext "basicConstraints=critical,CA:true" -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1 && openssl req -newkey rsa:2048 -nodes -keyout /out/private.key -out /tmp/leaf.csr -subj "/CN=trajectory-e2e.test" >/dev/null 2>&1 && printf "%s\n" "[v3_leaf]" "basicConstraints=critical,CA:false" "keyUsage=critical,digitalSignature,keyEncipherment" "extendedKeyUsage=serverAuth" "subjectAltName=DNS:trajectory-e2e.test,DNS:minio,DNS:localhost,IP:127.0.0.1" > /tmp/leaf.cnf && openssl x509 -req -in /tmp/leaf.csr -CA /out/ca.crt -CAkey /out/ca.key -CAcreateserial -out /out/public.crt -days 1 -extfile /tmp/leaf.cnf -extensions v3_leaf >/dev/null 2>&1 && rm -f /tmp/leaf.csr /tmp/leaf.cnf /out/ca.srl'
    }
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw 'Docker Desktop CLI is required for the full-stack E2E test.'
}
if (-not (Get-Command curl.exe -ErrorAction SilentlyContinue)) {
    throw 'curl.exe is required for the full-stack E2E test.'
}

New-TestCertificate -OutputDirectory $certificateDirectory
$env:E2E_CERT_DIR = (Resolve-Path -LiteralPath $certificateDirectory).Path
$env:E2E_PROXY_PORT = $proxyPort

try {
    Invoke-Compose up --build --detach

    $health = $null
    for ($attempt = 1; $attempt -le 60; $attempt++) {
        try {
            $health = Invoke-E2eCurl "https://${publicHostname}:${proxyPort}/api/v1/health"
            break
        }
        catch {
            Start-Sleep -Seconds 1
        }
    }
    if ($null -eq $health) {
        throw 'HTTPS proxy did not become healthy within 60 seconds.'
    }
    if (($health | ConvertFrom-Json).status -ne 'healthy') {
        throw 'Health endpoint did not report healthy.'
    }

    $dashboard = Invoke-E2eCurl "https://${publicHostname}:${proxyPort}/dashboard/"
    if ($dashboard -notmatch 'Trajectory') {
        throw 'Dashboard asset was not served through the TLS proxy.'
    }

    $archiveSha = ConvertTo-Sha256Hex $payload
    $registerBody = @{ machine_id = $machineId; hostname = 'trajectory-e2e-client'; os_version = 'Windows E2E'; registration_token = $enrollmentToken } | ConvertTo-Json -Compress
    $registration = Invoke-E2eCurl -H 'Content-Type: application/json' --data $registerBody "https://${publicHostname}:${proxyPort}/api/v1/machines/register" | ConvertFrom-Json
    if ([string]::IsNullOrWhiteSpace($registration.device_jwt)) {
        throw 'Machine registration did not return a device credential.'
    }
    $deviceJwt = $registration.device_jwt

    $heartbeatBody = @{ machine_id = $machineId; disk_usage_pct = 12.5; active_session_id = $sessionId } | ConvertTo-Json -Compress
    $heartbeat = Invoke-E2eCurl -H "Authorization: Bearer $deviceJwt" -H 'Content-Type: application/json' --data $heartbeatBody "https://${publicHostname}:${proxyPort}/api/v1/machines/heartbeat" | ConvertFrom-Json
    if ($heartbeat.status -ne 'ok') {
        throw 'Heartbeat was not persisted.'
    }

    $initiateBody = @{ session_id = $sessionId; chunk_count = 1; total_size_bytes = [System.Text.Encoding]::UTF8.GetByteCount($payload); archive_sha256 = $archiveSha; machine_id = $machineId; user_id = 'E2E_USER_0001'; schema_version = '1.0' } | ConvertTo-Json -Compress
    $initiated = Invoke-E2eCurl -H "Authorization: Bearer $deviceJwt" -H 'Content-Type: application/json' --data $initiateBody "https://${publicHostname}:${proxyPort}/api/v1/sessions" | ConvertFrom-Json
    if ($initiated.status -ne 'initiated') {
        throw 'Session initiation did not persist.'
    }

    $upload = Invoke-E2eCurl -X PUT -H "Authorization: Bearer $deviceJwt" -H "X-Chunk-SHA256: $archiveSha" --data-binary $payload "https://${publicHostname}:${proxyPort}/api/v1/sessions/${sessionId}/chunks/0" | ConvertFrom-Json
    if ($upload.status -ne 'stored' -or [string]::IsNullOrWhiteSpace($upload.storage_key)) {
        throw 'Chunk upload was not stored in S3.'
    }
    if ($upload.storage_key -notmatch '^[A-Za-z0-9_./-]+$') {
        throw 'Server returned an invalid object storage key.'
    }

    $completed = Invoke-E2eCurl -X POST -H "Authorization: Bearer $deviceJwt" "https://${publicHostname}:${proxyPort}/api/v1/sessions/${sessionId}/complete" | ConvertFrom-Json
    if ($completed.status -ne 'SESSION_ACCEPTED' -or $completed.archive_sha256_verified -ne $true) {
        throw 'Server did not verify the completed archive.'
    }

    $cookieJar = Join-Path $runDirectory 'dashboard.cookies'
    Invoke-E2eCurl -c $cookieJar -X POST -H 'Content-Type: application/json' --data (@{ password = $dashboardPassword } | ConvertTo-Json -Compress) "https://${publicHostname}:${proxyPort}/api/v1/dashboard/login" | Out-Null
    $machines = Invoke-E2eCurl -b $cookieJar "https://${publicHostname}:${proxyPort}/api/v1/machines" | ConvertFrom-Json
    if ($machines.machines.Count -ne 1 -or $machines.machines[0].machine_id -ne $machineId -or $machines.machines[0].status -ne 'ONLINE') {
        throw 'Dashboard machine presence does not reflect the persisted heartbeat.'
    }

    # sqlx migrations run at server startup; this asserts their PostgreSQL state is usable.
    $dbRow = Invoke-Compose exec -T postgres psql -U trajectory_e2e -d trajectory_e2e -Atqc "SELECT status || ':' || received_chunks || ':' || verified_sha256 FROM sessions WHERE session_id = '${sessionId}'"
    if (($dbRow | Select-Object -Last 1).Trim() -ne 'ACCEPTED:1:true') {
        throw 'PostgreSQL migration-backed session state was not accepted.'
    }

    # MinIO is queried separately so this proves a real TLS S3 write, not only API metadata.
    Invoke-Compose run --rm --no-deps --entrypoint /bin/sh minio-init -ec "mc alias set --insecure e2e https://minio:9000 \"`$MINIO_ROOT_USER\" \"`$MINIO_ROOT_PASSWORD\" >/dev/null && mc stat --insecure \"e2e/`$S3_BUCKET/$($upload.storage_key)\" >/dev/null"

    Write-Host 'Full-stack Docker E2E passed: PostgreSQL migrations, MinIO S3/TLS, server, HTTPS proxy, dashboard, and upload verification.'
}
finally {
    try {
        Invoke-Compose down --volumes --remove-orphans | Out-Null
    }
    catch {
        Write-Warning "Could not clean up disposable Compose project ${projectName}: $($_.Exception.Message)"
    }
    if (-not $KeepArtifacts) {
        Remove-Item -LiteralPath $runDirectory -Recurse -Force -ErrorAction SilentlyContinue
    }
}
