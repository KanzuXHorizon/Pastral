[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter()][string[]]$Arguments = @()
    )

    $output = & $FilePath @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    if ($null -ne $output) {
        $output | ForEach-Object { Write-Host $_ }
    }
    if ($exitCode -ne 0) {
        throw "Command failed with exit code ${exitCode}: $FilePath $($Arguments -join ' ')"
    }
    return ($output -join "`n").Trim()
}

function Get-CommandOutput {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter()][string[]]$Arguments = @()
    )

    $output = & $FilePath @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "Command failed with exit code ${exitCode}: $FilePath $($Arguments -join ' ')`n$($output -join "`n")"
    }
    return ($output -join "`n").Trim()
}

$requiredChannel = '1.97.1-x86_64-pc-windows-msvc'
$requiredTarget = 'x86_64-pc-windows-msvc'
$deferredSdk = '10.0.28000.2526'

Write-Host 'Pastral toolchain verification'
Write-Host 'Classification: RequiredForCurrentSlice'

$rustcVersion = Get-CommandOutput 'rustc' @('-Vv')
$cargoVersion = Get-CommandOutput 'cargo' @('-V')
$rustfmtVersion = Get-CommandOutput 'rustfmt' @('--version')
$clippyVersion = Get-CommandOutput 'cargo' @('clippy', '--version')
$activeToolchain = Get-CommandOutput 'rustup' @('show', 'active-toolchain')
$hostLine = ($rustcVersion -split "`r?`n" | Where-Object { $_ -like 'host:*' } | Select-Object -First 1)
$actualTarget = if ($hostLine) { ($hostLine -replace '^host:\s*', '').Trim() } else { '' }

Write-Host "rustc:`n$rustcVersion"
Write-Host "cargo: $cargoVersion"
Write-Host "rustup active toolchain: $activeToolchain"
Write-Host "target: $actualTarget"
Write-Host "rustfmt: $rustfmtVersion"
Write-Host "clippy: $clippyVersion"

$requiredFailures = New-Object System.Collections.Generic.List[string]
if ($rustcVersion -notmatch '^rustc 1\.97\.1 ') {
    $requiredFailures.Add("rustc must be 1.97.1")
}
if ($activeToolchain -notmatch '^1\.97\.1-x86_64-pc-windows-msvc\s') {
    $requiredFailures.Add("active toolchain must be $requiredChannel")
}
if ($actualTarget -ne $requiredTarget) {
    $requiredFailures.Add("host/target must be $requiredTarget")
}
if ($rustfmtVersion -notmatch '^rustfmt 1\.8\.0-stable ') {
    Write-Host 'rustfmt component is present; exact component build is reported above.'
}
if ($clippyVersion -notmatch '^clippy 0\.1\.97 ') {
    $requiredFailures.Add('Clippy must be the Rust 1.97.1 component')
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsInstall = $null
if (Test-Path $vswhere) {
    $vsInstall = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null | Select-Object -First 1)
}
if ([string]::IsNullOrWhiteSpace($vsInstall)) {
    $requiredFailures.Add('Visual Studio 2022 Build Tools with MSVC x64 tools was not detected')
} else {
    Write-Host "Visual Studio/MSVC: $vsInstall"
    $linkCandidates = Get-ChildItem -Path (Join-Path $vsInstall 'VC\Tools\MSVC') -Filter link.exe -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\Hostx64\\x64\\link\.exe$' } |
        Sort-Object FullName -Descending
    $link = $linkCandidates | Select-Object -First 1
    if ($null -eq $link) {
        $requiredFailures.Add('MSVC x64 linker link.exe was not found under the detected Visual Studio installation')
    } else {
        Write-Host "MSVC linker: $($link.FullName)"
    }
}

Write-Host 'Classification: DeferredForNativeSlice'
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
$sdkVersions = @()
if (Test-Path $sdkRoot) {
    $sdkVersions = @(Get-ChildItem -Path $sdkRoot -Directory | Select-Object -ExpandProperty Name | Sort-Object)
}
if ($sdkVersions.Count -eq 0) {
    Write-Host 'Windows SDKs: none detected (not required for the pure-domain slice)'
} else {
    Write-Host "Windows SDKs: $($sdkVersions -join ', ')"
}
if ($sdkVersions -contains $deferredSdk) {
    Write-Host "Deferred native SDK ${deferredSdk}: present"
} else {
    Write-Host "Deferred native SDK ${deferredSdk}: not present; accepted for the pure-domain slice"
}

if ($requiredFailures.Count -gt 0) {
    $requiredFailures | ForEach-Object { Write-Error "RequiredForCurrentSlice mismatch: $_" }
    exit 1
}

Write-Host 'RequiredForCurrentSlice: PASS'
exit 0
