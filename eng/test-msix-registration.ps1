[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$StagingDirectory,
    [Parameter()][string]$IdentityName = 'Pastral.Development'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw $Message
}

function Wait-ForNewProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][int[]]$BaselineIds,
        [Parameter()][int]$TimeoutSeconds = 20
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $process = Get-Process -Name $Name -ErrorAction SilentlyContinue |
            Where-Object { $BaselineIds -notcontains $_.Id } |
            Select-Object -First 1
        if ($null -ne $process) {
            return $process
        }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    Fail "Process was not created: $Name"
}

function Wait-ForWindow {
    param(
        [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Process,
        [Parameter()][int]$TimeoutSeconds = 20
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        if ($Process.HasExited) {
            Fail "Process exited before creating a window: $($Process.ProcessName)"
        }
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    Fail "Process did not create a window: $($Process.ProcessName)"
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

if (-not (Test-Path -LiteralPath $StagingDirectory -PathType Container)) {
    Fail "MSIX staging directory does not exist: $StagingDirectory"
}
$StagingDirectory = (Resolve-Path -LiteralPath $StagingDirectory).Path
$manifestPath = Join-Path $StagingDirectory 'AppxManifest.xml'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    Fail "Staging manifest does not exist: $manifestPath"
}

$existingProcesses = @(Get-Process -Name 'pastral-agent', 'pastral-manager' -ErrorAction SilentlyContinue)
$foreignProcesses = @($existingProcesses | Where-Object {
    [string]::IsNullOrWhiteSpace($_.Path) -or
    -not $_.Path.StartsWith($StagingDirectory, [StringComparison]::OrdinalIgnoreCase)
})
if ($foreignProcesses.Count -ne 0) {
    Fail 'Close all non-staging Pastral agent and manager processes before registration smoke'
}
foreach ($process in $existingProcesses) {
    $process.Kill()
    $process.WaitForExit()
    $process.Dispose()
}

$dataRoot = Join-Path $env:LOCALAPPDATA 'Pastral'
$backupRoot = Join-Path $env:LOCALAPPDATA ('Pastral.__smoke_backup_' + [guid]::NewGuid().ToString('N'))
$hadDataRoot = Test-Path -LiteralPath $dataRoot
$package = $null
$agent = $null
$manager = $null

try {
    Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue |
        Remove-AppxPackage -ErrorAction SilentlyContinue

    if ($hadDataRoot) {
        Move-Item -LiteralPath $dataRoot -Destination $backupRoot
    }

    Add-AppxPackage -Register $manifestPath
    $package = Get-AppxPackage -Name $IdentityName -ErrorAction Stop
    if ($package.Architecture -ne 'X64') {
        Fail "Registered package architecture is not x64: $($package.Architecture)"
    }
    if ((Resolve-Path -LiteralPath $package.InstallLocation).Path -ne $StagingDirectory) {
        Fail 'Registered package location does not match the staging directory'
    }

    $manifest = Get-AppxPackageManifest -Package $package.PackageFullName
    $application = @($manifest.Package.Applications.Application) |
        Where-Object { $_.Id -eq 'PastralManager' } |
        Select-Object -First 1
    if ($null -eq $application -or
        $application.Executable -ne 'pastral-manager.exe' -or
        $application.RuntimeBehavior -ne 'packagedClassicApp' -or
        $application.TrustLevel -ne 'mediumIL') {
        Fail 'Registered manager application declaration is invalid'
    }
    $startup = @($application.Extensions.Extension) |
        Where-Object { $_.Category -eq 'windows.startupTask' -and $_.Executable -eq 'pastral-agent.exe' } |
        Select-Object -First 1
    if ($null -eq $startup -or
        $startup.StartupTask.TaskId -ne 'PastralAgentStartup' -or
        [string]$startup.StartupTask.Enabled -ne 'true') {
        Fail 'Registered resident startup task declaration is invalid'
    }

    $agentExecutable = Join-Path $package.InstallLocation 'pastral-agent.exe'
    $agent = Start-Process -FilePath $agentExecutable -ArgumentList @(
        'run',
        '--max-connections',
        '2'
    ) -PassThru
    Start-Sleep -Milliseconds 750
    $agent.Refresh()
    if ($agent.HasExited) {
        Fail "Registered resident agent exited before manager activation: $($agent.ExitCode)"
    }

    $baselineIds = @(Get-Process -Name 'pastral-manager' -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Id)
    Start-Process explorer.exe -ArgumentList (
        'shell:AppsFolder\{0}!PastralManager' -f $package.PackageFamilyName
    )
    $manager = Wait-ForNewProcess -Name 'pastral-manager' -BaselineIds $baselineIds
    $windowHandle = Wait-ForWindow -Process $manager

    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $automationRoot = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
    if ($null -eq $automationRoot) {
        Fail 'UI Automation could not resolve the registered manager root'
    }
    [void](Find-AutomationElementByName -Root $automationRoot -Name 'Pastral agent is connected')
    [void](Find-AutomationElementByName -Root $automationRoot -Name '0 items' -TimeoutSeconds 5)

    [void]$manager.CloseMainWindow()
    if (-not $manager.WaitForExit(5000)) {
        $manager.Kill()
        $manager.WaitForExit()
        Fail 'Registered manager did not close within five seconds'
    }
    if (-not $agent.WaitForExit(10000)) {
        Fail 'Registered resident agent did not exit after two authenticated manager requests'
    }
}
finally {
    foreach ($process in @($manager, $agent)) {
        if ($null -ne $process) {
            if (-not $process.HasExited) {
                $process.Kill()
                $process.WaitForExit()
            }
            $process.Dispose()
        }
    }
    foreach ($staleProcess in @(Get-Process -Name 'pastral-agent', 'pastral-manager' -ErrorAction SilentlyContinue)) {
        if (-not [string]::IsNullOrWhiteSpace($staleProcess.Path) -and
            $staleProcess.Path.StartsWith($StagingDirectory, [StringComparison]::OrdinalIgnoreCase)) {
            $staleProcess.Kill()
            $staleProcess.WaitForExit()
        }
        $staleProcess.Dispose()
    }

    Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue |
        Remove-AppxPackage -ErrorAction SilentlyContinue

    Remove-Item -LiteralPath $dataRoot -Recurse -Force -ErrorAction SilentlyContinue
    if ($hadDataRoot -and (Test-Path -LiteralPath $backupRoot)) {
        Move-Item -LiteralPath $backupRoot -Destination $dataRoot
    }
}

if (Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue) {
    Fail 'Loose development package remained registered after smoke'
}
if ($hadDataRoot -and -not (Test-Path -LiteralPath $dataRoot)) {
    Fail 'Existing Pastral data root was not restored after smoke'
}
if (-not $hadDataRoot -and (Test-Path -LiteralPath $dataRoot)) {
    Fail 'Smoke data root remained after cleanup'
}

Write-Host 'Pastral packaged registration, Start Apps activation, live IPC, and cleanup smoke: PASS'
