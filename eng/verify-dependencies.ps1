[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Invoke-CargoTree {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $output = & cargo tree --locked @Arguments --prefix none --format '{p}'
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
    return @($output | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-PackageNames {
    param([Parameter(Mandatory = $true)][string[]]$Tree)

    return @(
        $Tree |
            ForEach-Object {
                $line = $_.Trim()
                if ($line.Length -gt 0) {
                    ($line -split '\s+')[0]
                }
            } |
            Sort-Object -Unique
    )
}

function Assert-NoPackages {
    param(
        [Parameter(Mandatory = $true)][string]$Scope,
        [Parameter(Mandatory = $true)][string[]]$Names,
        [Parameter(Mandatory = $true)][string[]]$Forbidden
    )

    $violations = @($Names | Where-Object { $Forbidden -contains $_ })
    if ($violations.Count -gt 0) {
        Write-Error ("Forbidden dependencies in ${Scope}: " + ($violations -join ', '))
        exit 1
    }
}

function Assert-ExactPackageVersion {
    param(
        [Parameter(Mandatory = $true)][string]$Scope,
        [Parameter(Mandatory = $true)][string[]]$Tree,
        [Parameter(Mandatory = $true)][string]$Package,
        [Parameter(Mandatory = $true)][string]$ExpectedLine
    )

    $matches = @(
        $Tree |
            Where-Object { $_ -match ('^' + [System.Text.RegularExpressions.Regex]::Escape($Package) + '\s+v') } |
            Sort-Object -Unique
    )
    if ($matches.Count -ne 1 -or $matches[0] -ne $ExpectedLine) {
        Write-Error ("Unexpected ${Package} version in ${Scope}: " + ($matches -join ', '))
        exit 1
    }
}

Write-Host 'Pastral dependency verification'

$workspaceTree = Invoke-CargoTree -Arguments @('--workspace')
$workspaceTree | ForEach-Object { Write-Host $_ }
$workspaceNames = Get-PackageNames -Tree $workspaceTree

$globalForbidden = @(
    'tokio',
    'serde',
    'chrono',
    'time',
    'sqlx',
    'prost',
    'prost-build',
    'prost-types',
    'tonic',
    'tonic-build',
    'grpcio',
    'serde_json',
    'bincode',
    'rkyv',
    'flatbuffers',
    'capnp',
    'tracing',
    'log',
    'reqwest',
    'hyper',
    'windows',
    'windows-core',
    'windows-app',
    'winui'
)
Assert-NoPackages -Scope 'workspace' -Names $workspaceNames -Forbidden $globalForbidden

$protobufPackages = @(
    'protobuf',
    'protobuf-codegen',
    'protobuf-macros',
    'linkme',
    'linkme-impl',
    'paste-complete'
)
foreach ($package in @(
    'pastral-agent',
    'pastral-agent-core',
    'pastral-clipboard-win',
    'pastral-domain',
    'pastral-ipc-auth',
    'pastral-ipc-core',
    'pastral-storage'
)) {
    $tree = Invoke-CargoTree -Arguments @('-p', $package, '--edges', 'all')
    Assert-NoPackages -Scope $package -Names (Get-PackageNames -Tree $tree) -Forbidden $protobufPackages
}

$schemaTree = Invoke-CargoTree -Arguments @('-p', 'pastral-ipc-schema', '--edges', 'all')
Assert-ExactPackageVersion -Scope 'pastral-ipc-schema' -Tree $schemaTree -Package 'protobuf' -ExpectedLine 'protobuf v4.35.0-release'
Assert-ExactPackageVersion -Scope 'pastral-ipc-schema' -Tree $schemaTree -Package 'protobuf-codegen' -ExpectedLine 'protobuf-codegen v4.35.0-release'
Assert-ExactPackageVersion -Scope 'pastral-ipc-schema' -Tree $schemaTree -Package 'protobuf-macros' -ExpectedLine 'protobuf-macros v4.35.0-release (proc-macro)'

$ipcWinTree = Invoke-CargoTree -Arguments @('-p', 'pastral-ipc-win', '--edges', 'all')
Assert-ExactPackageVersion -Scope 'pastral-ipc-win' -Tree $ipcWinTree -Package 'protobuf' -ExpectedLine 'protobuf v4.35.0-release'
Assert-ExactPackageVersion -Scope 'pastral-ipc-win' -Tree $ipcWinTree -Package 'protobuf-codegen' -ExpectedLine 'protobuf-codegen v4.35.0-release'
Assert-ExactPackageVersion -Scope 'pastral-ipc-win' -Tree $ipcWinTree -Package 'protobuf-macros' -ExpectedLine 'protobuf-macros v4.35.0-release (proc-macro)'

$ipcPrototypeForbidden = @(
    'tokio',
    'async-std',
    'smol',
    'mio',
    'socket2',
    'prost',
    'prost-build',
    'prost-types',
    'tonic',
    'tonic-build',
    'grpcio',
    'serde',
    'serde_json',
    'reqwest',
    'hyper',
    'h2',
    'tower',
    'axum',
    'tracing',
    'log'
)
foreach ($package in @('pastral-ipc-schema', 'pastral-ipc-probe', 'pastral-ipc-transport-probe', 'pastral-ipc-win')) {
    $tree = Invoke-CargoTree -Arguments @('-p', $package, '--edges', 'all')
    Assert-NoPackages -Scope $package -Names (Get-PackageNames -Tree $tree) -Forbidden $ipcPrototypeForbidden
}

$nonWindowsForbidden = @(
    'windows',
    'windows-core',
    'windows-sys',
    'windows-link',
    'windows-targets',
    'windows_aarch64_gnullvm',
    'windows_aarch64_msvc',
    'windows_i686_gnu',
    'windows_i686_gnullvm',
    'windows_i686_msvc',
    'windows_x86_64_gnu',
    'windows_x86_64_gnullvm',
    'windows_x86_64_msvc'
)
foreach ($package in @('pastral-agent-core', 'pastral-domain', 'pastral-ipc-auth', 'pastral-ipc-core', 'pastral-ipc-schema', 'pastral-ipc-probe', 'pastral-storage')) {
    $tree = Invoke-CargoTree -Arguments @('-p', $package)
    Assert-NoPackages -Scope $package -Names (Get-PackageNames -Tree $tree) -Forbidden $nonWindowsForbidden
}

foreach ($package in @('pastral-clipboard-win', 'pastral-agent', 'pastral-ipc-win')) {
    $tree = Invoke-CargoTree -Arguments @('-p', $package)
    $windowsLines = @(
        $tree | Where-Object {
            $_ -match '^(windows-sys|windows-link|windows-targets|windows_[A-Za-z0-9_]+)\s+v'
        }
    )
    $unexpectedWindowsBinding = @(
        $windowsLines | Where-Object {
            ($_ -notmatch '^windows-sys\s+v0\.61\.2$') -and
            ($_ -notmatch '^windows-link\s+v0\.2\.1$')
        }
    )
    if ($unexpectedWindowsBinding.Count -gt 0) {
        Write-Error ("Unexpected Windows binding packages in ${package}: " + ($unexpectedWindowsBinding -join ', '))
        exit 1
    }
    if (-not ($windowsLines -contains 'windows-sys v0.61.2')) {
        Write-Error "${package} is missing pinned windows-sys v0.61.2"
        exit 1
    }
}

$authTree = Invoke-CargoTree -Arguments @('-p', 'pastral-ipc-auth', '--edges', 'all')
Assert-ExactPackageVersion -Scope 'pastral-ipc-auth' -Tree $authTree -Package 'hmac' -ExpectedLine 'hmac v0.12.1'
Assert-ExactPackageVersion -Scope 'pastral-ipc-auth' -Tree $authTree -Package 'zeroize' -ExpectedLine 'zeroize v1.8.2'

Write-Host 'Dependency policy: PASS'
Write-Host 'Official protobuf 4.35.0-release is isolated to ipc-schema/ipc-probe/ipc-transport-probe/ipc-win; agent/domain/storage/clipboard/ipc-auth/ipc-core remain protobuf-free.'
Write-Host 'Agent-core/domain/ipc-auth/ipc-core/ipc-schema/ipc-probe/storage remain Windows-binding free; agent/clipboard-win/ipc-transport-probe/ipc-win use only pinned windows-sys/windows-link bindings.'
Write-Host 'Note: libsqlite3-sys may include build-helper crates such as cc, pkg-config, and vcpkg; no external vcpkg installation or manifest is required by the bundled SQLite build.'
