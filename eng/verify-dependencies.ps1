[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

Write-Host 'Pastral dependency verification'

$tree = & cargo tree --locked --workspace --prefix none --format '{p}'
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$tree | ForEach-Object { Write-Host $_ }

$forbidden = @(
    'tokio',
    'serde',
    'chrono',
    'time',
    'sqlx',
    'prost',
    'protobuf',
    'tracing',
    'log',
    'reqwest',
    'hyper',
    'windows',
    'windows-sys',
    'windows-core',
    'windows-targets',
    'windows-app',
    'winui'
)

$packageNames = @(
    $tree |
        ForEach-Object {
            $line = $_.Trim()
            if ($line.Length -gt 0) {
                ($line -split '\s+')[0]
            }
        } |
        Sort-Object -Unique
)

$violations = @($packageNames | Where-Object { $forbidden -contains $_ })
if ($violations.Count -gt 0) {
    Write-Error ("Forbidden foundation dependencies: " + ($violations -join ', '))
    exit 1
}

Write-Host 'Dependency policy: PASS'
Write-Host 'Note: libsqlite3-sys may include build-helper crates such as cc, pkg-config, and vcpkg; no external vcpkg installation or manifest is required by the bundled SQLite build.'
