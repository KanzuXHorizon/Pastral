[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$StagingDirectory,
    [Parameter()][string]$PackagePath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'package-toolchain.ps1')

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw $Message
}

function Assert-File {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    $path = Join-Path $StagingDirectory $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Fail "Required MSIX staging file is missing: $RelativePath"
    }
}

if (-not (Test-Path -LiteralPath $StagingDirectory -PathType Container)) {
    Fail "MSIX staging directory does not exist: $StagingDirectory"
}
$StagingDirectory = (Resolve-Path -LiteralPath $StagingDirectory).Path

$required = @(
    'AppxManifest.xml',
    'pastral-manager.exe',
    'pastral-manager-ipc-bridge.dll',
    'pastral-manager.pri',
    'Pastral.Manager.winmd',
    'pastral-agent.exe',
    'App.xbf',
    'MainWindow.xbf',
    'Pages\HomePage.xbf',
    'Pages\HistoryPage.xbf',
    'Themes\PastralTheme.xbf',
    'Assets\StoreLogo.png',
    'Assets\Square44x44Logo.png',
    'Assets\Square150x150Logo.png',
    'Assets\Wide310x150Logo.png',
    'Assets\Square310x310Logo.png',
    'Assets\SplashScreen.png'
)
$required | ForEach-Object { Assert-File $_ }

$forbiddenNames = @(
    'pastral-agent-ipc.exe',
    'pastral-agent-ipc-probe.exe',
    'pastral-manager-ipc-bridge.pdb',
    'Microsoft.WindowsAppRuntime.Bootstrap.dll',
    'Microsoft.Web.WebView2.Core.dll',
    'Microsoft.Web.WebView2.Core.Projection.dll',
    'Microsoft.Web.WebView2.Core.winmd'
)
foreach ($name in $forbiddenNames) {
    if (Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File |
        Where-Object { $_.Name -ieq $name }) {
        Fail "Forbidden diagnostic or symbol file was staged: $name"
    }
}

$forbiddenExtensions = @('.pdb', '.lib', '.exp', '.pfx', '.p12', '.pem', '.key', '.log')
foreach ($file in Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File) {
    if ($forbiddenExtensions -contains $file.Extension.ToLowerInvariant()) {
        Fail "Forbidden file type was staged: $($file.FullName)"
    }
}

$manifestPath = Join-Path $StagingDirectory 'AppxManifest.xml'
$manifestText = [System.IO.File]::ReadAllText($manifestPath)
if ($manifestText.Contains('@IDENTITY_NAME@') -or
    $manifestText.Contains('@PUBLISHER@') -or
    $manifestText.Contains('@VERSION@')) {
    Fail 'MSIX manifest still contains unresolved template tokens'
}

[xml]$manifest = $manifestText
$namespace = New-Object System.Xml.XmlNamespaceManager($manifest.NameTable)
$namespace.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
$namespace.AddNamespace('uap10', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10')
$namespace.AddNamespace('desktop', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10')
$namespace.AddNamespace('rescap', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')

$identity = $manifest.SelectSingleNode('/f:Package/f:Identity', $namespace)
if ($null -eq $identity -or $identity.ProcessorArchitecture -ne 'x64') {
    Fail 'MSIX identity is missing or is not x64'
}
if ($identity.Publisher -notlike 'CN=*') {
    Fail 'MSIX publisher must be an explicit certificate subject'
}
$app = $manifest.SelectSingleNode('/f:Package/f:Applications/f:Application[@Id="PastralManager"]', $namespace)
if ($null -eq $app -or $app.Executable -ne 'pastral-manager.exe') {
    Fail 'PastralManager application declaration is invalid'
}
$runtimeBehavior = $app.GetAttribute('RuntimeBehavior', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10')
$trustLevel = $app.GetAttribute('TrustLevel', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10')
if ($runtimeBehavior -ne 'packagedClassicApp' -or $trustLevel -ne 'mediumIL') {
    Fail 'Manager must be a packagedClassicApp at medium integrity'
}
$startup = $manifest.SelectSingleNode(
    '/f:Package/f:Applications/f:Application/f:Extensions/desktop:Extension[@Category="windows.startupTask" and @Executable="pastral-agent.exe"]/desktop:StartupTask[@TaskId="PastralAgentStartup"]',
    $namespace
)
if ($null -eq $startup -or $startup.Enabled -ne 'true') {
    Fail 'Pastral resident startup task is missing or disabled'
}
$fullTrust = $manifest.SelectSingleNode('/f:Package/f:Capabilities/rescap:Capability[@Name="runFullTrust"]', $namespace)
if ($null -eq $fullTrust) {
    Fail 'MSIX runFullTrust capability is missing'
}
foreach ($dependencyName in @('Microsoft.WindowsAppRuntime.2', 'Microsoft.VCLibs.140.00.UWPDesktop')) {
    $dependency = $manifest.SelectSingleNode(
        "/f:Package/f:Dependencies/f:PackageDependency[@Name='$dependencyName']",
        $namespace
    )
    if ($null -eq $dependency) {
        Fail "MSIX dependency is missing: $dependencyName"
    }
}

$allowedRootFiles = @(
    'App.xbf',
    'AppxManifest.xml',
    'MainWindow.xbf',
    'Pastral.Manager.winmd',
    'pastral-agent.exe',
    'pastral-manager-ipc-bridge.dll',
    'pastral-manager.exe',
    'pastral-manager.pri'
)
foreach ($file in Get-ChildItem -LiteralPath $StagingDirectory -File) {
    if ($allowedRootFiles -notcontains $file.Name) {
        Fail "Unexpected root staging file: $($file.Name)"
    }
}
$allowedDirectories = @('Assets', 'Pages', 'Themes')
foreach ($directory in Get-ChildItem -LiteralPath $StagingDirectory -Directory) {
    if ($allowedDirectories -notcontains $directory.Name) {
        Fail "Unexpected staging directory: $($directory.Name)"
    }
}

if (-not [string]::IsNullOrWhiteSpace($PackagePath)) {
    if (-not (Test-Path -LiteralPath $PackagePath -PathType Leaf)) {
        Fail "MSIX package does not exist: $PackagePath"
    }
    $makeAppx = Resolve-PastralWindowsSdkTool `
        -RepositoryRoot $repositoryRoot `
        -Name 'makeappx.exe'
    $unpackRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('pastral-msix-unpack-' + [guid]::NewGuid().ToString('N'))
    try {
        & $makeAppx unpack /p $PackagePath /d $unpackRoot /o
        if ($LASTEXITCODE -ne 0) {
            Fail "MakeAppx unpack failed with exit code $LASTEXITCODE"
        }
        foreach ($source in Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File) {
            $relative = $source.FullName.Substring($StagingDirectory.Length).TrimStart('\')
            $unpacked = Join-Path $unpackRoot $relative
            if (-not (Test-Path -LiteralPath $unpacked -PathType Leaf)) {
                Fail "Packaged file is missing after extraction: $relative"
            }
            $sourceHash = (Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash
            $unpackedHash = (Get-FileHash -LiteralPath $unpacked -Algorithm SHA256).Hash
            if ($sourceHash -ne $unpackedHash) {
                Fail "Packaged file hash mismatch after extraction: $relative"
            }
        }
    }
    finally {
        Remove-Item -LiteralPath $unpackRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host 'Pastral MSIX layout verification: PASS'
