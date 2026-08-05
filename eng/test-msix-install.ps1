[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackagePath,
    [Parameter(Mandatory = $true)][string]$CertificatePath,
    [Parameter()][string]$IdentityName = 'Pastral.Development'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw $Message
}

function Wait-ForProcessWindow {
    param(
        [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Process,
        [Parameter()][int]$TimeoutSeconds = 20
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline -and -not $Process.HasExited) {
        Start-Sleep -Milliseconds 200
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
    }
    if ($Process.HasExited) {
        Fail "Process exited before creating a window: $($Process.ExitCode)"
    }
    Fail 'Process did not create a top-level window within the timeout'
}

function Find-AutomationElementByName {
    param(
        [Parameter(Mandatory = $true)]$Root,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter()][int]$TimeoutSeconds = 15
    )
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        $Name
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $element = $Root.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $condition
        )
        if ($null -ne $element) {
            return $element
        }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $deadline)
    Fail "UI Automation element was not found: $Name"
}

if (-not (Test-Path -LiteralPath $PackagePath -PathType Leaf)) {
    Fail "MSIX package does not exist: $PackagePath"
}
if (-not (Test-Path -LiteralPath $CertificatePath -PathType Leaf)) {
    Fail "Development certificate does not exist: $CertificatePath"
}
$PackagePath = (Resolve-Path -LiteralPath $PackagePath).Path
$CertificatePath = (Resolve-Path -LiteralPath $CertificatePath).Path

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail 'Signed MSIX install smoke requires an elevated PowerShell session so the certificate can be trusted in LocalMachine\TrustedPeople'
}

$certificate = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($CertificatePath)
$thumbprint = $certificate.Thumbprint
$trustedPeoplePath = "Cert:\LocalMachine\TrustedPeople\$thumbprint"
$trustedPeopleAlreadyPresent = Test-Path -LiteralPath $trustedPeoplePath

$existingPackage = Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue
if ($null -ne $existingPackage) {
    foreach ($package in @($existingPackage)) {
        Remove-AppxPackage -Package $package.PackageFullName
    }
}

$installedPackage = $null
$agentProcess = $null
$managerProcess = $null
$shellManagerProcess = $null
$oldDiagnostic = $env:PASTRAL_MANAGER_DIAGNOSTIC
$oldDataRoot = $env:PASTRAL_MANAGER_DATA_ROOT
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('pastral-msix-smoke-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null

try {
    if (-not $trustedPeopleAlreadyPresent) {
        Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null
    }

    Add-AppxPackage -Path $PackagePath
    $installedPackage = Get-AppxPackage -Name $IdentityName -ErrorAction Stop
    if ($installedPackage.Architecture -ne 'X64') {
        Fail "Installed package architecture is not x64: $($installedPackage.Architecture)"
    }

    $manifest = Get-AppxPackageManifest -Package $installedPackage.PackageFullName
    $application = @($manifest.Package.Applications.Application) |
        Where-Object { $_.Id -eq 'PastralManager' } |
        Select-Object -First 1
    if ($null -eq $application -or $application.Executable -ne 'pastral-manager.exe') {
        Fail 'Installed manifest does not contain the PastralManager application'
    }
    $startupExtension = @($application.Extensions.Extension) |
        Where-Object { $_.Category -eq 'windows.startupTask' -and $_.Executable -eq 'pastral-agent.exe' } |
        Select-Object -First 1
    if ($null -eq $startupExtension -or
        $startupExtension.StartupTask.TaskId -ne 'PastralAgentStartup' -or
        [string]$startupExtension.StartupTask.Enabled -ne 'true') {
        Fail 'Installed manifest does not contain the enabled Pastral resident startup task'
    }

    $managerExecutable = Join-Path $installedPackage.InstallLocation 'pastral-manager.exe'
    $agentExecutable = Join-Path $installedPackage.InstallLocation 'pastral-agent.exe'
    foreach ($path in @($managerExecutable, $agentExecutable, (Join-Path $installedPackage.InstallLocation 'pastral-manager-ipc-bridge.dll'))) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            Fail "Installed package payload is missing: $path"
        }
    }

    $baselineManagerIds = @(Get-Process -Name 'pastral-manager' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
    Start-Process explorer.exe -ArgumentList ("shell:AppsFolder\{0}!PastralManager" -f $installedPackage.PackageFamilyName)
    $shellDeadline = [DateTime]::UtcNow.AddSeconds(20)
    while ([DateTime]::UtcNow -lt $shellDeadline -and $null -eq $shellManagerProcess) {
        Start-Sleep -Milliseconds 250
        $shellManagerProcess = Get-Process -Name 'pastral-manager' -ErrorAction SilentlyContinue |
            Where-Object { $baselineManagerIds -notcontains $_.Id } |
            Select-Object -First 1
    }
    if ($null -eq $shellManagerProcess) {
        Fail 'Installed Start menu application activation did not create the manager process'
    }
    $shellHandle = Wait-ForProcessWindow -Process $shellManagerProcess
    [void]$shellManagerProcess.CloseMainWindow()
    if (-not $shellManagerProcess.WaitForExit(5000)) {
        $shellManagerProcess.Kill()
        $shellManagerProcess.WaitForExit()
        Fail 'Shell-activated manager did not close within five seconds'
    }
    $shellManagerProcess.Dispose()
    $shellManagerProcess = $null

    $env:PASTRAL_MANAGER_DIAGNOSTIC = '1'
    $env:PASTRAL_MANAGER_DATA_ROOT = $temporaryRoot
    $agentProcess = Start-Process `
        -FilePath $agentExecutable `
        -ArgumentList @('run', '--data-root', $temporaryRoot, '--max-connections', '2') `
        -PassThru
    Start-Sleep -Milliseconds 750
    $agentProcess.Refresh()
    if ($agentProcess.HasExited) {
        Fail "Installed resident agent exited before manager launch: $($agentProcess.ExitCode)"
    }

    $managerProcess = Start-Process -FilePath $managerExecutable -PassThru
    $windowHandle = Wait-ForProcessWindow -Process $managerProcess
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $automationRoot = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
    if ($null -eq $automationRoot) {
        Fail 'UI Automation could not resolve the installed manager root'
    }
    [void](Find-AutomationElementByName -Root $automationRoot -Name 'Pastral agent is connected' -TimeoutSeconds 15)
    [void](Find-AutomationElementByName -Root $automationRoot -Name '0 items' -TimeoutSeconds 5)

    [void]$managerProcess.CloseMainWindow()
    if (-not $managerProcess.WaitForExit(5000)) {
        $managerProcess.Kill()
        $managerProcess.WaitForExit()
        Fail 'Installed manager did not close within five seconds'
    }
    if (-not $agentProcess.WaitForExit(10000)) {
        Fail 'Installed resident agent did not stop after the bounded Health and History connections'
    }
}
finally {
    $env:PASTRAL_MANAGER_DIAGNOSTIC = $oldDiagnostic
    $env:PASTRAL_MANAGER_DATA_ROOT = $oldDataRoot

    foreach ($process in @($managerProcess, $shellManagerProcess, $agentProcess)) {
        if ($null -ne $process) {
            if (-not $process.HasExited) {
                $process.Kill()
                $process.WaitForExit()
            }
            $process.Dispose()
        }
    }

    $package = Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue
    if ($null -ne $package) {
        foreach ($item in @($package)) {
            Remove-AppxPackage -Package $item.PackageFullName -ErrorAction SilentlyContinue
        }
    }
    Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    if (-not $trustedPeopleAlreadyPresent) {
        Remove-Item -LiteralPath $trustedPeoplePath -Force -ErrorAction SilentlyContinue
    }
}

if (Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue) {
    Fail 'Development package remained installed after uninstall smoke'
}
Write-Host 'Pastral MSIX install, activation, live IPC, and uninstall smoke: PASS'
