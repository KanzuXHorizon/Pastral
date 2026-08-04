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
    'protobuf',
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
foreach ($package in @('pastral-agent-core', 'pastral-domain', 'pastral-storage')) {
    $tree = Invoke-CargoTree -Arguments @('-p', $package)
    Assert-NoPackages -Scope $package -Names (Get-PackageNames -Tree $tree) -Forbidden $nonWindowsForbidden
}

$clipboardTree = Invoke-CargoTree -Arguments @('-p', 'pastral-clipboard-win')
$clipboardWindowsLines = @(
    $clipboardTree | Where-Object {
        $_ -match '^(windows-sys|windows-link|windows-targets|windows_[A-Za-z0-9_]+)\s+v'
    }
)
$unexpectedWindowsBinding = @(
    $clipboardWindowsLines | Where-Object {
        ($_ -notmatch '^windows-sys\s+v0\.61\.2$') -and
        ($_ -notmatch '^windows-link\s+v0\.2\.1$')
    }
)
if ($unexpectedWindowsBinding.Count -gt 0) {
    Write-Error ('Unexpected clipboard Windows binding packages: ' + ($unexpectedWindowsBinding -join ', '))
    exit 1
}
if (-not ($clipboardWindowsLines -contains 'windows-sys v0.61.2')) {
    Write-Error 'pastral-clipboard-win is missing pinned windows-sys v0.61.2'
    exit 1
}

Write-Host 'Dependency policy: PASS'
Write-Host 'Agent-core/domain/storage remain Windows-binding free; clipboard-win uses only pinned windows-sys/windows-link bindings.'
Write-Host 'Note: libsqlite3-sys may include build-helper crates such as cc, pkg-config, and vcpkg; no external vcpkg installation or manifest is required by the bundled SQLite build.'
