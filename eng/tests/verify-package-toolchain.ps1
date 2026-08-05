[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $repositoryRoot 'eng\package-toolchain.ps1')

function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]$Expected,
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if ($Expected -ne $Actual) {
        throw "$Message. Expected '$Expected', actual '$Actual'."
    }
}

$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('pastral-package-toolchain-' + [guid]::NewGuid().ToString('N'))
try {
    $fakeRepository = Join-Path $temporaryRoot 'repository'
    $fakeSdk = Join-Path $temporaryRoot 'sdk'
    New-Item -ItemType Directory -Path $fakeRepository, $fakeSdk -Force | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $fakeRepository 'Directory.Build.props'),
        @'
<?xml version="1.0" encoding="utf-8"?>
<Project>
  <PropertyGroup>
    <WindowsTargetPlatformVersion>10.0.26100.0</WindowsTargetPlatformVersion>
  </PropertyGroup>
</Project>
'@,
        [System.Text.UTF8Encoding]::new($false)
    )

    $exactDirectory = Join-Path $fakeSdk 'bin\10.0.26100.0\x64'
    $higherDirectory = Join-Path $fakeSdk 'bin\10.0.99999.0\x64'
    New-Item -ItemType Directory -Path $exactDirectory, $higherDirectory -Force | Out-Null
    $exactTool = Join-Path $exactDirectory 'makeappx.exe'
    $higherTool = Join-Path $higherDirectory 'makeappx.exe'
    [System.IO.File]::WriteAllBytes($exactTool, [byte[]](1))
    [System.IO.File]::WriteAllBytes($higherTool, [byte[]](2))

    Assert-Equal '10.0.26100.0' (Get-PastralWindowsSdkVersion -RepositoryRoot $fakeRepository) 'Configured SDK version was not read exactly'
    Assert-Equal $exactTool (Resolve-PastralWindowsSdkTool -RepositoryRoot $fakeRepository -SdkRoot $fakeSdk -Name 'makeappx.exe') 'Resolver selected a different installed SDK'

    Remove-Item -LiteralPath $exactTool -Force
    $failedClosed = $false
    try {
        [void](Resolve-PastralWindowsSdkTool -RepositoryRoot $fakeRepository -SdkRoot $fakeSdk -Name 'makeappx.exe')
    }
    catch {
        $failedClosed = $_.Exception.Message.Contains('10.0.26100.0')
    }
    if (-not $failedClosed) {
        throw 'Resolver did not fail closed when only a different SDK version was installed.'
    }

    foreach ($scriptName in @('build-msix.ps1', 'verify-msix-layout.ps1')) {
        $scriptPath = Join-Path $repositoryRoot ("eng\{0}" -f $scriptName)
        $scriptText = [System.IO.File]::ReadAllText($scriptPath)
        if ($scriptText -notmatch 'package-toolchain\.ps1' -or
            $scriptText -notmatch 'Resolve-PastralWindowsSdkTool') {
            throw "$scriptName does not use the exact shared Windows SDK tool resolver."
        }
        if ($scriptText -match 'Sort-Object\s+FullName\s+-Descending') {
            throw "$scriptName still selects the highest installed Windows SDK."
        }
    }

    $buildScript = [System.IO.File]::ReadAllText((Join-Path $repositoryRoot 'eng\build-msix.ps1'))
    foreach ($reportMarker in @(
        'windows-sdk-version=',
        'makeappx-file-version=',
        'makeappx-sha256=',
        'signtool-file-version=',
        'signtool-sha256='
    )) {
        if (-not $buildScript.Contains($reportMarker)) {
            throw "build-msix.ps1 does not record package toolchain evidence: $reportMarker"
        }
    }

    Write-Host 'Package toolchain resolver tests: PASS'
}
finally {
    Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
}
