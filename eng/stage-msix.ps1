[CmdletBinding()]
param(
    [Parameter()][string]$Version,
    [Parameter()][string]$IdentityName = 'Pastral.Development',
    [Parameter()][string]$Publisher = 'CN=Pastral Development',
    [Parameter()][string]$StagingDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $repositoryRoot 'target\package'
$managerProject = Join-Path $repositoryRoot 'apps\manager\Pastral.Manager\Pastral.Manager.vcxproj'
$manifestTemplate = Join-Path $repositoryRoot 'packaging\Pastral\AppxManifest.xml.in'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw $Message
}

function Resolve-MSBuild {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        Fail 'Visual Studio Installer vswhere.exe was not found'
    }
    $installation = & $vswhere -latest -products * -requires `
        Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
        -property installationPath
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($installation)) {
        Fail 'Visual Studio with x64 MSVC and Windows App SDK build tools was not found'
    }
    $msbuild = Join-Path $installation 'MSBuild\Current\Bin\MSBuild.exe'
    if (-not (Test-Path -LiteralPath $msbuild -PathType Leaf)) {
        Fail "MSBuild.exe was not found at $msbuild"
    }
    return $msbuild
}

function Copy-RequiredFile {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        Fail "Required package input is missing: $Source"
    }
    $destinationDirectory = Split-Path -Parent $Destination
    if (-not [string]::IsNullOrWhiteSpace($destinationDirectory)) {
        New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $cargoText = [System.IO.File]::ReadAllText((Join-Path $repositoryRoot 'Cargo.toml'))
    $match = [System.Text.RegularExpressions.Regex]::Match(
        $cargoText,
        '(?ms)^\[workspace\.package\].*?^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"'
    )
    if (-not $match.Success) {
        Fail 'Workspace package version could not be read from Cargo.toml'
    }
    $Version = $match.Groups['version'].Value + '.0'
}
elseif ($Version -match '^\d+\.\d+\.\d+$') {
    $Version += '.0'
}
if ($Version -notmatch '^\d+\.\d+\.\d+\.\d+$') {
    Fail 'MSIX version must contain three or four numeric components'
}
if ($IdentityName -notmatch '^[A-Za-z0-9.-]{3,50}$') {
    Fail 'MSIX identity name contains unsupported characters or length'
}
if ($Publisher -notmatch '^CN=.+') {
    Fail 'MSIX publisher must be an explicit certificate subject beginning with CN='
}

if ([string]::IsNullOrWhiteSpace($StagingDirectory)) {
    $StagingDirectory = Join-Path $targetRoot ("Pastral_{0}_x64" -f $Version)
}
$managerOutput = Join-Path $targetRoot 'manager-release'
$managerIntermediate = Join-Path $targetRoot 'manager-release-obj'

New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
Remove-Item -LiteralPath $StagingDirectory -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $managerOutput -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $managerIntermediate -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $StagingDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $managerOutput -Force | Out-Null
New-Item -ItemType Directory -Path $managerIntermediate -Force | Out-Null

Push-Location $repositoryRoot
try {
    Write-Host 'Building resident agent Release'
    & cargo build --locked -p pastral-agent --release
    if ($LASTEXITCODE -ne 0) {
        Fail "Resident agent Release build failed with exit code $LASTEXITCODE"
    }

    Write-Host 'Building native manager Release'
    $msbuild = Resolve-MSBuild
    $managerOutputArgument = $managerOutput.TrimEnd('\') + '\'
    $managerIntermediateArgument = $managerIntermediate.TrimEnd('\') + '\'
    & $msbuild $managerProject '/restore' '/m:1' '/nr:false' '/nologo' '/verbosity:minimal' `
        '/p:Configuration=Release' '/p:Platform=x64' '/p:RestoreLockedMode=true' `
        '/p:WindowsPackageType=MSIX' `
        '/p:WindowsAppSdkBootstrapInitialize=false' `
        '/p:WindowsAppSdkDeploymentManagerInitialize=false' `
        '/p:WindowsAppSdkUndockedRegFreeWinRTInitialize=false' `
        "/p:OutDir=$managerOutputArgument" "/p:IntDir=$managerIntermediateArgument"
    if ($LASTEXITCODE -ne 0) {
        Fail "Native manager Release build failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

$rootRuntimeFiles = @(
    'App.xbf',
    'MainWindow.xbf',
    'Pastral.Manager.winmd',
    'pastral-manager-ipc-bridge.dll',
    'pastral-manager.exe',
    'pastral-manager.pri'
)
foreach ($relative in $rootRuntimeFiles) {
    Copy-RequiredFile `
        -Source (Join-Path $managerOutput $relative) `
        -Destination (Join-Path $StagingDirectory $relative)
}
foreach ($relative in @(
    'Pages\HomePage.xbf',
    'Pages\HistoryPage.xbf',
    'Themes\PastralTheme.xbf'
)) {
    Copy-RequiredFile `
        -Source (Join-Path $managerOutput $relative) `
        -Destination (Join-Path $StagingDirectory $relative)
}
Copy-RequiredFile `
    -Source (Join-Path $repositoryRoot 'target\release\pastral-agent.exe') `
    -Destination (Join-Path $StagingDirectory 'pastral-agent.exe')

& (Join-Path $PSScriptRoot 'generate-package-assets.ps1') `
    -OutputDirectory (Join-Path $StagingDirectory 'Assets')

if (-not (Test-Path -LiteralPath $manifestTemplate -PathType Leaf)) {
    Fail "MSIX manifest template is missing: $manifestTemplate"
}
$manifest = [System.IO.File]::ReadAllText($manifestTemplate)
$manifest = $manifest.Replace('@IDENTITY_NAME@', $IdentityName)
$manifest = $manifest.Replace('@PUBLISHER@', $Publisher)
$manifest = $manifest.Replace('@VERSION@', $Version)
[System.IO.File]::WriteAllText(
    (Join-Path $StagingDirectory 'AppxManifest.xml'),
    $manifest,
    [System.Text.UTF8Encoding]::new($false)
)

& (Join-Path $PSScriptRoot 'verify-msix-layout.ps1') -StagingDirectory $StagingDirectory

Write-Host "Pastral MSIX staging complete: $StagingDirectory"
Write-Output (Resolve-Path -LiteralPath $StagingDirectory).Path
