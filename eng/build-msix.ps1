[CmdletBinding()]
param(
    [Parameter()][string]$Version,
    [Parameter()][string]$IdentityName = 'Pastral.Development',
    [Parameter()][string]$Publisher = 'CN=Pastral Development',
    [Parameter()][string]$OutputDirectory,
    [Parameter()][string]$PfxPath,
    [Parameter()][SecureString]$PfxPassword,
    [Parameter()][switch]$CreateDevelopmentCertificate,
    [Parameter()][switch]$PreserveDevelopmentPfx
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot 'artifacts'
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw $Message
}

function Resolve-SdkTool {
    param([Parameter(Mandatory = $true)][string]$Name)
    $sdkRoot = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots').KitsRoot10
    $tool = Get-ChildItem (Join-Path $sdkRoot 'bin') -Recurse -Filter $Name |
        Where-Object { $_.FullName -match "\\x64\\$([System.Text.RegularExpressions.Regex]::Escape($Name))$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if ([string]::IsNullOrWhiteSpace($tool)) {
        Fail "$Name x64 was not found in the Windows SDK"
    }
    return $tool
}

function ConvertTo-PlainText {
    param([Parameter(Mandatory = $true)][SecureString]$Value)
    $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($Value)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
    }
    finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer)
    }
}

$stageArguments = @{
    IdentityName = $IdentityName
    Publisher = $Publisher
}
if (-not [string]::IsNullOrWhiteSpace($Version)) {
    $stageArguments.Version = $Version
}
$stagingOutput = @(& (Join-Path $PSScriptRoot 'stage-msix.ps1') @stageArguments)
$StagingDirectory = $stagingOutput | Select-Object -Last 1
if ([string]::IsNullOrWhiteSpace($StagingDirectory) -or
    -not (Test-Path -LiteralPath $StagingDirectory -PathType Container)) {
    Fail 'MSIX staging did not return a valid directory'
}

[xml]$manifest = Get-Content -LiteralPath (Join-Path $StagingDirectory 'AppxManifest.xml') -Raw
$packageVersion = [string]$manifest.Package.Identity.Version
$packagePublisher = [string]$manifest.Package.Identity.Publisher
if ($packagePublisher -ne $Publisher) {
    Fail 'Rendered manifest publisher does not match the requested publisher'
}

$packageName = "Pastral-$packageVersion-x64.msix"
$packagePath = Join-Path $OutputDirectory $packageName
$certificatePath = Join-Path $OutputDirectory 'Pastral-Development.cer'
$checksumPath = $packagePath + '.sha256'
$reportPath = Join-Path $OutputDirectory "Pastral-$packageVersion-x64-verification.txt"
Remove-Item -LiteralPath $packagePath, $certificatePath, $checksumPath, $reportPath -Force -ErrorAction SilentlyContinue

$makeAppx = Resolve-SdkTool 'makeappx.exe'
$signTool = Resolve-SdkTool 'signtool.exe'

Write-Host "Packing $packageName"
& $makeAppx pack /v /o /h SHA256 /d $StagingDirectory /p $packagePath
if ($LASTEXITCODE -ne 0) {
    Fail "MakeAppx pack failed with exit code $LASTEXITCODE"
}
$temporaryCertificate = $null
$trustedCertificate = $null
$rootCertificate = $null
$generatedPfx = $false
$signingRoot = Join-Path $repositoryRoot 'target\package\signing'
New-Item -ItemType Directory -Path $signingRoot -Force | Out-Null

if ([string]::IsNullOrWhiteSpace($PfxPath)) {
    if (-not $CreateDevelopmentCertificate) {
        Fail 'Provide -PfxPath or explicitly request -CreateDevelopmentCertificate'
    }
    $generatedPfx = $true
    $PfxPath = Join-Path $signingRoot 'Pastral-Development.pfx'
    Remove-Item -LiteralPath $PfxPath -Force -ErrorAction SilentlyContinue

    $passwordBytes = New-Object byte[] 32
    $random = [Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $random.GetBytes($passwordBytes)
    }
    finally {
        $random.Dispose()
    }
    $passwordText = [Convert]::ToBase64String($passwordBytes)
    [Array]::Clear($passwordBytes, 0, $passwordBytes.Length)
    $PfxPassword = ConvertTo-SecureString $passwordText -AsPlainText -Force

    $temporaryCertificate = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $Publisher `
        -FriendlyName 'Pastral Development Package Signing' `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -KeyExportPolicy Exportable `
        -KeyAlgorithm RSA `
        -KeyLength 2048 `
        -HashAlgorithm SHA256 `
        -NotAfter (Get-Date).AddYears(2) `
        -TextExtension @(
            '2.5.29.37={text}1.3.6.1.5.5.7.3.3',
            '2.5.29.19={text}ca=0&pathlength=0'
        )
    if ($temporaryCertificate.Subject -ne $Publisher) {
        Fail "Generated certificate subject '$($temporaryCertificate.Subject)' does not match '$Publisher'"
    }
    Export-PfxCertificate `
        -Cert $temporaryCertificate `
        -FilePath $PfxPath `
        -Password $PfxPassword `
        -ChainOption EndEntityCertOnly | Out-Null
    Export-Certificate -Cert $temporaryCertificate -FilePath $certificatePath -Type CERT | Out-Null
}
else {
    if (-not (Test-Path -LiteralPath $PfxPath -PathType Leaf)) {
        Fail "Signing PFX does not exist: $PfxPath"
    }
    if ($null -eq $PfxPassword) {
        Fail 'A SecureString -PfxPassword is required with -PfxPath'
    }
    $pfxInfo = Get-PfxData -FilePath $PfxPath -Password $PfxPassword
    if ($pfxInfo.EndEntityCertificates.Count -ne 1) {
        Fail 'Signing PFX must contain exactly one end-entity certificate'
    }
    $certificate = $pfxInfo.EndEntityCertificates[0]
    if ($certificate.Subject -ne $Publisher) {
        Fail "Signing certificate subject '$($certificate.Subject)' does not match '$Publisher'"
    }
    Export-Certificate -Cert $certificate -FilePath $certificatePath -Type CERT | Out-Null
}

try {
    $plainPassword = ConvertTo-PlainText $PfxPassword
    try {
        Write-Host 'Signing MSIX with SHA-256'
        & $signTool sign /fd SHA256 /f $PfxPath /p $plainPassword $packagePath
        if ($LASTEXITCODE -ne 0) {
            Fail "SignTool sign failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        $plainPassword = $null
        if ($generatedPfx) {
            $passwordText = $null
        }
    }

    $trustedCertificate = Import-Certificate `
        -FilePath $certificatePath `
        -CertStoreLocation 'Cert:\CurrentUser\TrustedPeople'
    $rootCertificate = Import-Certificate `
        -FilePath $certificatePath `
        -CertStoreLocation 'Cert:\CurrentUser\Root'
    & $signTool verify /pa /all /v $packagePath
    if ($LASTEXITCODE -ne 0) {
        Fail "SignTool verification failed with exit code $LASTEXITCODE"
    }

    & (Join-Path $PSScriptRoot 'verify-msix-layout.ps1') `
        -StagingDirectory $StagingDirectory `
        -PackagePath $packagePath

    $packageHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $packageFile = Get-Item -LiteralPath $packagePath
    [System.IO.File]::WriteAllText(
        $checksumPath,
        "$packageHash *$($packageFile.Name)`r`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $report = @(
        'Pastral development MSIX verification',
        "version=$packageVersion",
        "identity=$IdentityName",
        "publisher=$Publisher",
        'architecture=x64',
        'manager=pastral-manager.exe',
        'resident=pastral-agent.exe',
        'startup-task=PastralAgentStartup',
        'makeappx-pack-validation=passed',
        'signature-verification=passed',
        'extraction-parity=passed',
        "package-bytes=$($packageFile.Length)",
        "sha256=$packageHash"
    )
    [System.IO.File]::WriteAllLines(
        $reportPath,
        $report,
        [System.Text.UTF8Encoding]::new($false)
    )
}
finally {
    if ($null -ne $trustedCertificate) {
        Remove-Item -LiteralPath ("Cert:\CurrentUser\TrustedPeople\" + $trustedCertificate.Thumbprint) `
            -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $rootCertificate) {
        Remove-Item -LiteralPath ("Cert:\CurrentUser\Root\" + $rootCertificate.Thumbprint) `
            -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $temporaryCertificate) {
        Remove-Item -LiteralPath ("Cert:\CurrentUser\My\" + $temporaryCertificate.Thumbprint) `
            -Force -ErrorAction SilentlyContinue
    }
    if ($generatedPfx -and -not $PreserveDevelopmentPfx) {
        Remove-Item -LiteralPath $PfxPath -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Pastral development installer created: $packagePath"
Write-Output $packagePath
Write-Output $certificatePath
Write-Output $checksumPath
Write-Output $reportPath
