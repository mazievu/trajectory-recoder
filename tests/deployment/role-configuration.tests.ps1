$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$validator = Join-Path $repositoryRoot 'deployment/Validate-RoleConfiguration.ps1'
$fixtures = Join-Path $PSScriptRoot 'fixtures'

function Assert-ValidationPasses {
    param(
        [string]$ConfigName,
        [string]$ExpectedRole
    )

    & $validator -ConfigPath (Join-Path $fixtures $ConfigName) -ExpectedRole $ExpectedRole
    if ($LASTEXITCODE -ne 0) {
        throw "Expected $ConfigName to pass $ExpectedRole validation, but it failed."
    }
}

function Assert-ValidationFails {
    param(
        [string]$ConfigName,
        [string]$ExpectedRole
    )

    & $validator -ConfigPath (Join-Path $fixtures $ConfigName) -ExpectedRole $ExpectedRole 2>$null
    if ($LASTEXITCODE -eq 0) {
        throw "Expected $ConfigName to fail $ExpectedRole validation, but it passed."
    }
}

Assert-ValidationPasses -ConfigName 'client-valid.env' -ExpectedRole 'client'
Assert-ValidationPasses -ConfigName 'server-valid.env' -ExpectedRole 'server'
Assert-ValidationFails -ConfigName 'client-missing-server-url.env' -ExpectedRole 'client'
Assert-ValidationFails -ConfigName 'client-loopback-server-url.env' -ExpectedRole 'client'
Assert-ValidationFails -ConfigName 'server-with-client-role.env' -ExpectedRole 'server'
Assert-ValidationFails -ConfigName 'server-missing-tls.env' -ExpectedRole 'server'

Write-Host 'Role configuration contract tests passed.'
