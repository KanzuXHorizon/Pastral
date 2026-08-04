[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Test', 'Probe', 'Live', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$managerRoot = Join-Path $repositoryRoot 'apps\manager\Pastral.Manager'
$managerProject = Join-Path $managerRoot 'Pastral.Manager.vcxproj'
$probeProject = Join-Path $managerRoot 'Tests\Pastral.Manager.IpcProbe.vcxproj'
$bridgeManifest = Join-Path $repositoryRoot 'crates\manager-ipc-bridge\Cargo.toml'
$bridgeDllSource = Join-Path $repositoryRoot 'target\release\pastral_manager_ipc_bridge.dll'
$bridgeDllName = 'pastral-manager-ipc-bridge.dll'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Error $Message
    exit 1
}

function Assert-File {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Required manager IPC bridge file is missing: $Path"
    }
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $content = [System.IO.File]::ReadAllText($Path)
    if (-not [System.Text.RegularExpressions.Regex]::IsMatch(
        $content,
        $Pattern,
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor
            [System.Text.RegularExpressions.RegexOptions]::Multiline
    )) {
        Fail "$Description is missing from $Path"
    }
}

function Resolve-VisualStudioInstallation {
    $vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
    Assert-File $vswhere
    $installationPath = (& $vswhere -latest -products * -requires `
        Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
        -property installationPath).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($installationPath)) {
        Fail 'Visual Studio 2022 with MSVC x64 and C++ WinUI build tools was not found'
    }
    return $installationPath
}

function Resolve-MSBuild {
    $path = Join-Path (Resolve-VisualStudioInstallation) 'MSBuild\Current\Bin\MSBuild.exe'
    Assert-File $path
    return $path
}

function Resolve-Dumpbin {
    $installation = Resolve-VisualStudioInstallation
    $tool = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Filter dumpbin.exe -Recurse -File |
        Where-Object { $_.FullName -match '\\bin\\Hostx64\\x64\\dumpbin\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($null -eq $tool) {
        Fail 'x64 dumpbin.exe was not found in the active Visual Studio installation'
    }
    return $tool.FullName
}

function Invoke-MSBuildProject {
    param(
        [Parameter(Mandatory = $true)][string]$Project,
        [Parameter(Mandatory = $true)][ValidateSet('Debug', 'Release')][string]$Configuration
    )

    $msbuild = Resolve-MSBuild
    & $msbuild $Project '/restore' '/m:1' '/nr:false' '/nologo' '/verbosity:minimal' `
        "/p:Configuration=$Configuration" '/p:Platform=x64' '/p:RestoreLockedMode=false'
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Read-SharedText {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return ''
    }
    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
    )
    try {
        $reader = New-Object System.IO.StreamReader($stream)
        try {
            return $reader.ReadToEnd()
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Wait-ForFileText {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $Path -PathType Leaf) {
            $content = Read-SharedText -Path $Path
            if ($content.Contains($Text)) {
                return
            }
        }
        Start-Sleep -Milliseconds 100
    }
    $observed = if (Test-Path -LiteralPath $Path -PathType Leaf) {
        Read-SharedText -Path $Path
    } else {
        '<no output>'
    }
    Fail "Timed out waiting for '$Text'. Observed: $observed"
}

function Start-AgentHealthServer {
    param(
        [Parameter(Mandatory = $true)][string]$DataRoot,
        [Parameter(Mandatory = $true)][int]$MaxConnections,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$ErrorPath
    )

    $agent = Join-Path $repositoryRoot 'target\release\pastral-agent-ipc.exe'
    Assert-File $agent
    $process = Start-Process -FilePath $agent -ArgumentList @(
        'serve-health', '--data-root', $DataRoot, '--max-connections', $MaxConnections
    ) -RedirectStandardOutput $OutputPath -RedirectStandardError $ErrorPath -PassThru
    Wait-ForFileText -Path $OutputPath -Text 'agent-ipc-ready=1' -TimeoutSeconds 15
    if ($process.HasExited) {
        $errorText = if (Test-Path -LiteralPath $ErrorPath) { Read-SharedText -Path $ErrorPath } else { '' }
        Fail "Agent Health server exited before serving a client: $errorText"
    }
    return $process
}

function Find-AutomationElementByName {
    param(
        [Parameter(Mandatory = $true)]$Root,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        $Name
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $element = $Root.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $condition)
        if ($null -ne $element) {
            return $element
        }
        Start-Sleep -Milliseconds 150
    }
    Fail "UI Automation could not find '$Name' within $TimeoutSeconds seconds"
}

function Invoke-StaticVerification {
    Write-Host 'Pastral manager IPC bridge static verification'
    foreach ($path in @(
        $bridgeManifest,
        (Join-Path $repositoryRoot 'crates\manager-ipc-bridge\src\ffi.rs'),
        (Join-Path $repositoryRoot 'crates\manager-ipc-bridge\include\pastral_manager_ipc_bridge.h'),
        (Join-Path $repositoryRoot 'apps\agent\src\ipc_health.rs'),
        (Join-Path $managerRoot 'Services\ManagerIpcBridge.cpp'),
        (Join-Path $managerRoot 'Services\IManagerDataProvider.h'),
        $managerProject,
        $probeProject,
        (Join-Path $repositoryRoot 'eng\build.ps1'),
        (Join-Path $repositoryRoot '.github\workflows\rust-ci.yml')
    )) {
        Assert-File $path
    }

    $loader = Join-Path $managerRoot 'Services\ManagerIpcBridge.cpp'
    Assert-Contains $loader 'GetModuleFileNameW' 'executable-directory resolution'
    Assert-Contains $loader 'LoadLibraryExW' 'secure DLL loading'
    Assert-Contains $loader 'LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR' 'DLL-local dependency search restriction'
    Assert-Contains $loader 'pastral-manager-ipc-bridge\.dll' 'exact deployed bridge name'

    $provider = Join-Path $managerRoot 'Services\IManagerDataProvider.h'
    Assert-Contains $provider 'LoadSnapshotAsync\(' 'asynchronous provider contract'

    Assert-Contains $managerProject 'BuildPastralManagerIpcBridge' 'manager Rust bridge build target'
    Assert-Contains $managerProject 'cargo build --locked -p pastral-manager-ipc-bridge' 'locked bridge cargo build'
    Assert-Contains $managerProject 'pastral_manager_ipc_bridge\.dll' 'Cargo bridge source name'
    Assert-Contains $managerProject 'pastral-manager-ipc-bridge\.dll' 'deployed bridge destination name'
    Assert-Contains $managerProject '<Copy\s+SourceFiles=' 'bridge output copy'

    $build = Join-Path $repositoryRoot 'eng\build.ps1'
    Assert-Contains $build "'ManagerIpcBridge'" 'ManagerIpcBridge build task'
    Assert-Contains $build 'verify-manager-ipc-bridge\.ps1' 'manager bridge verifier dispatch'

    $workflow = Join-Path $repositoryRoot '.github\workflows\rust-ci.yml'
    Assert-Contains $workflow 'crates/manager-ipc-bridge/\*\*' 'bridge CI path filter'
    Assert-Contains $workflow 'verify-manager-ipc-bridge\.ps1\s+-Mode\s+(Static|Test)' 'manager bridge CI verification'

    Write-Host 'Manager IPC bridge static policy: PASS'
}

function Invoke-TestVerification {
    Write-Host 'Testing feature-gated agent Health server'
    & cargo test --locked -p pastral-agent --features ipc-health --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host 'Testing manager IPC bridge'
    & cargo test --locked -p pastral-manager-ipc-bridge --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & cargo clippy --locked -p pastral-manager-ipc-bridge --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    & cargo build --locked -p pastral-manager-ipc-bridge --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Assert-File $bridgeDllSource

    $exports = @(& (Resolve-Dumpbin) /exports $bridgeDllSource 2>&1)
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $exportText = $exports -join "`n"
    foreach ($name in @(
        'pastral_manager_ipc_abi_version',
        'pastral_manager_ipc_result_size',
        'pastral_manager_ipc_health_w'
    )) {
        if (-not $exportText.Contains($name)) {
            Fail "Bridge DLL export is missing: $name"
        }
    }

    $tree = @(& cargo tree --locked -p pastral-manager-ipc-bridge --edges normal,build --prefix none --format '{p}')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $treeText = $tree -join "`n"
    foreach ($forbidden in @('tokio ', 'reqwest ', 'hyper ', 'serde_json ', 'rusqlite ')) {
        if ($treeText.Contains($forbidden)) {
            Fail "Manager bridge dependency tree contains forbidden package: $forbidden"
        }
    }

    Write-Host 'Manager IPC bridge Rust tests and exports: PASS'
}

function Invoke-ProbeVerification {
    Write-Host 'Building native manager IPC probe'
    & cargo build --locked -p pastral-manager-ipc-bridge --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & cargo build --locked -p pastral-agent --features ipc-health --bin pastral-agent-ipc --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Invoke-MSBuildProject -Project $probeProject -Configuration Release

    $probeOutput = Join-Path $managerRoot 'Tests\x64\Release'
    $probe = Join-Path $probeOutput 'Pastral.Manager.IpcProbe.exe'
    if (-not (Test-Path -LiteralPath $probe -PathType Leaf)) {
        $probe = Join-Path $probeOutput 'pastral-manager-ipc-probe.exe'
    }
    Assert-File $probe
    Copy-Item -LiteralPath $bridgeDllSource -Destination (Join-Path $probeOutput $bridgeDllName) -Force

    $abiOutput = @(& $probe --abi 2>&1)
    if ($LASTEXITCODE -ne 0 -or ($abiOutput -join "`n") -notmatch 'manager-ipc-abi=ok') {
        Fail "Native bridge ABI probe failed: $($abiOutput -join ' ')"
    }

    $temporary = Join-Path ([System.IO.Path]::GetTempPath()) ('pastral-manager-ipc-' + [guid]::NewGuid().ToString('N'))
    $dataRoot = Join-Path $temporary 'data'
    $stdout = Join-Path $temporary 'agent.out'
    $stderr = Join-Path $temporary 'agent.err'
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null
    $agent = $null
    try {
        $agent = Start-AgentHealthServer -DataRoot $dataRoot -MaxConnections 1 -OutputPath $stdout -ErrorPath $stderr
        $healthOutput = @(& $probe --health --data-root $dataRoot 2>&1)
        $healthExit = $LASTEXITCODE
        if ($healthExit -ne 0) {
            Fail "Native Health probe failed with code ${healthExit}: $($healthOutput -join ' ')"
        }
        $healthText = $healthOutput -join "`n"
        foreach ($marker in @(
            'manager-ipc-probe=ok',
            'status=0',
            'storage-schema=1',
            'privacy-policy-ok=1',
            'storage-integrity-ok=1'
        )) {
            if (-not $healthText.Contains($marker)) {
                Fail "Native Health probe output is missing: $marker"
            }
        }
        foreach ($forbidden in @('secret=', 'nonce=', 'proof=', '\\.\pipe\', $dataRoot.ToLowerInvariant())) {
            if ($healthText.ToLowerInvariant().Contains($forbidden.ToLowerInvariant())) {
                Fail "Native Health probe emitted forbidden content: $forbidden"
            }
        }
        if (-not $agent.WaitForExit(10000)) {
            Fail 'Agent Health server did not exit after the bounded probe connection'
        }
    }
    finally {
        if ($null -ne $agent) {
            if (-not $agent.HasExited) {
                $agent.Kill()
                $agent.WaitForExit()
            }
            $agent.Dispose()
        }
        Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host 'Native manager IPC bridge probe: PASS'
}

function Invoke-LiveVerification {
    Write-Host 'Building Release manager with deployed Rust bridge'
    & cargo build --locked -p pastral-agent --features ipc-health --bin pastral-agent-ipc --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Invoke-MSBuildProject -Project $managerProject -Configuration Release

    $managerOutput = Join-Path $managerRoot 'x64\Release'
    $manager = Join-Path $managerOutput 'pastral-manager.exe'
    Assert-File $manager
    Assert-File (Join-Path $managerOutput $bridgeDllName)

    $runtime = @(Get-AppxPackage -Name 'Microsoft.WindowsAppRuntime.2' -ErrorAction SilentlyContinue |
        Where-Object { $_.Architecture -eq 'X64' -and $_.Version -eq [version]'2.3.1.0' })
    if ($runtime.Count -eq 0) {
        Fail 'Microsoft.WindowsAppRuntime.2 x64 version 2.3.1.0 is required for live manager verification'
    }

    $temporary = Join-Path ([System.IO.Path]::GetTempPath()) ('pastral-manager-live-' + [guid]::NewGuid().ToString('N'))
    $dataRoot = Join-Path $temporary 'data'
    $stdout = Join-Path $temporary 'agent.out'
    $stderr = Join-Path $temporary 'agent.err'
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null

    $agent = $null
    $managerProcess = $null
    $oldDiagnostic = $env:PASTRAL_MANAGER_DIAGNOSTIC
    $oldRoot = $env:PASTRAL_MANAGER_DATA_ROOT
    try {
        $agent = Start-AgentHealthServer -DataRoot $dataRoot -MaxConnections 1 -OutputPath $stdout -ErrorPath $stderr
        $env:PASTRAL_MANAGER_DIAGNOSTIC = '1'
        $env:PASTRAL_MANAGER_DATA_ROOT = $dataRoot
        $managerProcess = Start-Process -FilePath $manager -PassThru

        $windowDeadline = [DateTime]::UtcNow.AddSeconds(15)
        $windowHandle = [IntPtr]::Zero
        while ([DateTime]::UtcNow -lt $windowDeadline -and -not $managerProcess.HasExited) {
            Start-Sleep -Milliseconds 200
            $managerProcess.Refresh()
            $windowHandle = $managerProcess.MainWindowHandle
            if ($windowHandle -ne [IntPtr]::Zero) { break }
        }
        if ($managerProcess.HasExited) {
            Fail "Release manager exited during live verification with code $($managerProcess.ExitCode)"
        }
        if ($windowHandle -eq [IntPtr]::Zero) {
            Fail 'Release manager did not create a top-level window within 15 seconds'
        }

        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
        if ($null -eq $root) { Fail 'UI Automation could not resolve the Release manager root' }

        [void](Find-AutomationElementByName -Root $root -Name 'Pastral agent is connected' -TimeoutSeconds 15)
        [void](Find-AutomationElementByName -Root $root -Name '0 items' -TimeoutSeconds 5)
        if (-not $agent.WaitForExit(10000)) {
            Fail 'Agent Health server did not exit after serving the manager connection'
        }

        $refresh = Find-AutomationElementByName -Root $root -Name 'Refresh local agent connection' -TimeoutSeconds 5
        $invoke = $refresh.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        [void](Find-AutomationElementByName -Root $root -Name 'Pastral agent is not connected' -TimeoutSeconds 10)
        [void](Find-AutomationElementByName -Root $root -Name '0 items' -TimeoutSeconds 5)

        [void]$managerProcess.CloseMainWindow()
        if (-not $managerProcess.WaitForExit(5000)) {
            Fail 'Release manager did not close within five seconds'
        }
    }
    finally {
        if ($null -ne $managerProcess) {
            if (-not $managerProcess.HasExited) {
                $managerProcess.Kill()
                $managerProcess.WaitForExit()
            }
            $managerProcess.Dispose()
        }
        if ($null -ne $agent) {
            if (-not $agent.HasExited) {
                $agent.Kill()
                $agent.WaitForExit()
            }
            $agent.Dispose()
        }
        $env:PASTRAL_MANAGER_DIAGNOSTIC = $oldDiagnostic
        $env:PASTRAL_MANAGER_DATA_ROOT = $oldRoot
        Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host 'Release manager live Connected-to-Disconnected UIA smoke: PASS'
}

Push-Location $repositoryRoot
try {
    switch ($Mode) {
        'Static' { Invoke-StaticVerification }
        'Test' { Invoke-TestVerification }
        'Probe' { Invoke-ProbeVerification }
        'Live' { Invoke-LiveVerification }
        'All' {
            Invoke-StaticVerification
            Invoke-TestVerification
            Invoke-ProbeVerification
            Invoke-LiveVerification
        }
    }
}
finally {
    Pop-Location
}
