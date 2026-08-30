[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$ConfigPath,

    [Parameter(Mandatory = $true)]
    [ValidateSet('client', 'server')]
    [string]$ExpectedRole
)

$ErrorActionPreference = 'Stop'

function Read-EnvironmentFile {
    param([string]$Path)

    $values = @{}
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $Path) {
        $lineNumber++
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith('#')) {
            continue
        }

        $separator = $trimmed.IndexOf('=')
        if ($separator -lt 1) {
            throw "Invalid environment assignment at line $lineNumber in $Path."
        }

        $name = $trimmed.Substring(0, $separator).Trim()
        $value = $trimmed.Substring($separator + 1).Trim()
        if ($values.ContainsKey($name)) {
            throw "Duplicate environment variable '$name' in $Path."
        }
        $values[$name] = $value
    }
    return $values
}

function Require-Values {
    param(
        [hashtable]$Values,
        [string[]]$Names,
        [System.Collections.Generic.List[string]]$Errors
    )

    foreach ($name in $Names) {
        if (-not $Values.ContainsKey($name) -or [string]::IsNullOrWhiteSpace($Values[$name])) {
            $Errors.Add("Missing required variable: $name")
        }
    }
}

function Reject-Values {
    param(
        [hashtable]$Values,
        [string[]]$Names,
        [System.Collections.Generic.List[string]]$Errors
    )

    foreach ($name in $Names) {
        if ($Values.ContainsKey($name) -and -not [string]::IsNullOrWhiteSpace($Values[$name])) {
            $Errors.Add("$name is not valid for this deployment role")
        }
    }
}

$values = Read-EnvironmentFile -Path $ConfigPath
$errors = [System.Collections.Generic.List[string]]::new()

if (-not $values.ContainsKey('DEPLOYMENT_ROLE')) {
    $errors.Add('Missing required variable: DEPLOYMENT_ROLE')
} elseif (-not $values['DEPLOYMENT_ROLE'].Equals($ExpectedRole, [System.StringComparison]::OrdinalIgnoreCase)) {
    $errors.Add("DEPLOYMENT_ROLE must be '$ExpectedRole'")
}

switch ($ExpectedRole) {
    'client' {
        Require-Values -Values $values -Names @(
            'TRAJECTORY_SERVER_URL', 'TRAJECTORY_MACHINE_ID', 'TRAJECTORY_USER_ID', 'SPOOL_DIR'
        ) -Errors $errors
        if ($values.ContainsKey('SPOOL_DIR') -and -not [string]::IsNullOrWhiteSpace($values['SPOOL_DIR']) -and -not [System.IO.Path]::IsPathRooted($values['SPOOL_DIR'])) {
            $errors.Add('SPOOL_DIR must be an absolute path')
        }
        Reject-Values -Values $values -Names @(
            'BIND_ADDR', 'DATABASE_URL', 'S3_ENDPOINT', 'S3_BUCKET', 'S3_REGION',
            'S3_ACCESS_KEY', 'S3_SECRET_KEY', 'JWT_SECRET', 'ENROLLMENT_TOKEN',
            'DASHBOARD_API_TOKEN', 'SERVER_URL'
        ) -Errors $errors

        if ((-not $values.ContainsKey('DEVICE_TOKEN') -or [string]::IsNullOrWhiteSpace($values['DEVICE_TOKEN'])) -and
            (-not $values.ContainsKey('TRAJECTORY_ENROLLMENT_TOKEN') -or [string]::IsNullOrWhiteSpace($values['TRAJECTORY_ENROLLMENT_TOKEN']))) {
            $credentialCachePath = if ($values.ContainsKey('TRAJECTORY_DEVICE_TOKEN_PATH') -and -not [string]::IsNullOrWhiteSpace($values['TRAJECTORY_DEVICE_TOKEN_PATH'])) {
                $values['TRAJECTORY_DEVICE_TOKEN_PATH']
            } else {
                Join-Path $values['SPOOL_DIR'] 'device-token.dpapi'
            }
            if (-not (Test-Path -LiteralPath $credentialCachePath -PathType Leaf)) {
                $errors.Add('A client requires DEVICE_TOKEN, TRAJECTORY_ENROLLMENT_TOKEN, or an existing DPAPI device-token cache')
            }
        }

        if ($values.ContainsKey('TRAJECTORY_SERVER_URL') -and -not [string]::IsNullOrWhiteSpace($values['TRAJECTORY_SERVER_URL'])) {
            $serverUrl = $null
            if (-not [Uri]::TryCreate($values['TRAJECTORY_SERVER_URL'], [UriKind]::Absolute, [ref]$serverUrl)) {
                $errors.Add('TRAJECTORY_SERVER_URL must be an absolute HTTPS URL')
            } elseif ($serverUrl.Scheme -ne 'https') {
                $errors.Add('TRAJECTORY_SERVER_URL must use HTTPS')
            } elseif ($serverUrl.IsLoopback) {
                $errors.Add('TRAJECTORY_SERVER_URL must not point to a loopback address')
            }
        }
    }
    'server' {
        Require-Values -Values $values -Names @(
            'BIND_ADDR', 'PUBLIC_HOSTNAME', 'TLS_CERT_PATH', 'TLS_KEY_PATH',
            'DATABASE_URL', 'S3_ENDPOINT', 'S3_BUCKET', 'S3_REGION',
            'S3_ACCESS_KEY', 'S3_SECRET_KEY', 'JWT_SECRET', 'ENROLLMENT_TOKEN',
            'DASHBOARD_API_TOKEN',
            'POSTGRES_DB', 'POSTGRES_USER', 'POSTGRES_PASSWORD',
            'MINIO_ROOT_USER', 'MINIO_ROOT_PASSWORD'
        ) -Errors $errors
        Reject-Values -Values $values -Names @(
            'TRAJECTORY_SERVER_URL', 'TRAJECTORY_MACHINE_ID', 'TRAJECTORY_USER_ID',
            'TRAJECTORY_ENROLLMENT_TOKEN', 'DEVICE_TOKEN', 'SPOOL_DIR'
        ) -Errors $errors

        if ($values.ContainsKey('BIND_ADDR') -and $values['BIND_ADDR'] -match '^(127\.0\.0\.1|localhost|\[::1\]):') {
            $errors.Add('BIND_ADDR must not be loopback when accepting remote clients')
        }
        if ($values.ContainsKey('PUBLIC_HOSTNAME') -and $values['PUBLIC_HOSTNAME'] -match '://|[/:\\\s]') {
            $errors.Add('PUBLIC_HOSTNAME must be a hostname only, without scheme, path, port, or whitespace')
        }
        if ($values.ContainsKey('S3_ENDPOINT') -and -not [string]::IsNullOrWhiteSpace($values['S3_ENDPOINT'])) {
            $objectStoreUrl = $null
            if (-not [Uri]::TryCreate($values['S3_ENDPOINT'], [UriKind]::Absolute, [ref]$objectStoreUrl) -or $objectStoreUrl.Scheme -ne 'https') {
                $errors.Add('S3_ENDPOINT must be an absolute HTTPS URL')
            }
        }
    }
}

if ($errors.Count -gt 0) {
    foreach ($errorMessage in $errors) {
        [Console]::Error.WriteLine($errorMessage)
    }
    exit 1
}

Write-Host "Validated $ExpectedRole deployment configuration: $ConfigPath"
exit 0
