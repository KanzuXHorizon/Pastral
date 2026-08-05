[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$buildPath = Join-Path $repositoryRoot 'eng\build.ps1'
$buildText = [System.IO.File]::ReadAllText($buildPath)
$fullMatch = [System.Text.RegularExpressions.Regex]::Match(
    $buildText,
    "(?ms)^\s*'Full'\s*\{(?<body>.*?)^\s*\}\s*$"
)
if (-not $fullMatch.Success) {
    throw 'eng/build.ps1 does not contain a parseable Full task body.'
}
$fullBody = $fullMatch.Groups['body'].Value
foreach ($required in @(
    'Invoke-VerifyNative',
    'Invoke-PackageToolchain',
    'Invoke-Agent',
    'Invoke-ManagerIpcBridge',
    'Invoke-Manager'
)) {
    if ($fullBody -notmatch ("(?m)^\s*{0}\s*$" -f [regex]::Escape($required))) {
        throw "Full task does not invoke the canonical gate: $required"
    }
}
foreach ($obsolete in @('Invoke-NativePolicy', 'Invoke-ManagerBuild')) {
    if ($fullBody -match ("(?m)^\s*{0}\s*$" -f [regex]::Escape($obsolete))) {
        throw "Full task duplicates a partial native gate instead of using Invoke-Manager: $obsolete"
    }
}

Write-Host 'Build dispatch verification: PASS'
