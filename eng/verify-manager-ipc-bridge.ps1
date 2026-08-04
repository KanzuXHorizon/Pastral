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
$nativeBuildMutexName = 'Local\Pastral.NativeManager.Build'

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

function Invoke-WithNativeBuildLock {
    param([Parameter(Mandatory = $true)][scriptblock]$Action)

    $mutex = [System.Threading.Mutex]::new($false, $nativeBuildMutexName)
    $acquired = $false
    try {
        try {
            $acquired = $mutex.WaitOne([TimeSpan]::FromMinutes(10))
        }
        catch [System.Threading.AbandonedMutexException] {
            $acquired = $true
        }
        if (-not $acquired) {
            Fail "Timed out waiting for native manager build lock: $nativeBuildMutexName"
        }
        & $Action
    }
    finally {
        if ($acquired) {
            [void]$mutex.ReleaseMutex()
        }
        $mutex.Dispose()
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

function Assert-ExactBridgeExports {
    param([Parameter(Mandatory = $true)][string]$Path)

    $output = @(& (Resolve-Dumpbin) /nologo /exports $Path 2>&1)
    if ($LASTEXITCODE -ne 0) {
        Fail "dumpbin export inspection failed for $Path"
    }

    $actual = @(
        $output | ForEach-Object {
            if ($_ -match '^\s*\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(\S+)') {
                $Matches[1]
            }
        } | Sort-Object -Unique
    )
    $expected = @(
        '__Disallow_Upb_And_Cpp_In_Same_Binary',
        'pastral_manager_ipc_abi_version',
        'pastral_manager_ipc_clip_item_size',
        'pastral_manager_ipc_health_w',
        'pastral_manager_ipc_history_w',
        'pastral_manager_ipc_read_abi_version',
        'pastral_manager_ipc_read_result_size',
        'pastral_manager_ipc_result_size',
        'pastral_manager_ipc_search_w'
    ) | Sort-Object

    $difference = @(Compare-Object -ReferenceObject $expected -DifferenceObject $actual)
    if ($difference.Count -ne 0 -or $actual.Count -ne $expected.Count) {
        Fail "Unexpected manager IPC bridge exports: $($actual -join ', ')"
    }
}

function Invoke-MSBuildProject {
    param(
        [Parameter(Mandatory = $true)][string]$Project,
        [Parameter(Mandatory = $true)][ValidateSet('Debug', 'Release')][string]$Configuration,
        [Parameter(Mandatory = $true)][string]$OutputDirectory,
        [Parameter(Mandatory = $true)][string]$IntermediateDirectory
    )

    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    New-Item -ItemType Directory -Path $IntermediateDirectory -Force | Out-Null
    $output = $OutputDirectory.TrimEnd('\') + '\'
    $intermediate = $IntermediateDirectory.TrimEnd('\') + '\'

    $msbuild = Resolve-MSBuild
    Invoke-WithNativeBuildLock {
        & $msbuild $Project '/restore' '/m:1' '/nr:false' '/nologo' '/verbosity:quiet' `
            "/p:Configuration=$Configuration" '/p:Platform=x64' '/p:RestoreLockedMode=false' `
            "/p:OutDir=$output" "/p:IntDir=$intermediate"
        if ($LASTEXITCODE -ne 0) {
            throw "Native project build failed with exit code $LASTEXITCODE"
        }
    }
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

function Start-AgentServer {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('serve-health', 'serve-read')][string]$Command,
        [Parameter(Mandatory = $true)][string]$DataRoot,
        [Parameter(Mandatory = $true)][int]$MaxConnections,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$ErrorPath
    )

    $agent = Join-Path $repositoryRoot 'target\release\pastral-agent-ipc.exe'
    Assert-File $agent
    $process = Start-Process -FilePath $agent -ArgumentList @(
        $Command, '--data-root', $DataRoot, '--max-connections', $MaxConnections
    ) -RedirectStandardOutput $OutputPath -RedirectStandardError $ErrorPath -PassThru
    Wait-ForFileText -Path $OutputPath -Text 'agent-ipc-ready=1' -TimeoutSeconds 15
    if ($process.HasExited) {
        $errorText = if (Test-Path -LiteralPath $ErrorPath) { Read-SharedText -Path $ErrorPath } else { '' }
        Fail "Agent IPC server '$Command' exited before serving a client: $errorText"
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
    Assert-Contains $loader 'IsReadAvailable' 'independent read bridge availability'
    Assert-Contains $loader 'pastral_manager_ipc_history_w' 'History bridge export loading'
    Assert-Contains $loader 'pastral_manager_ipc_search_w' 'Search bridge export loading'
    Assert-Contains $loader 'MaxReadItems\s*=\s*100' 'bounded read item capacity'
    Assert-Contains $loader 'MaxTextBytes\s*=\s*256\s*\*\s*1024' 'bounded read text capacity'

    $provider = Join-Path $managerRoot 'Services\IManagerDataProvider.h'
    Assert-Contains $provider 'LoadSnapshotAsync\(' 'asynchronous provider contract'

    Assert-Contains $PSCommandPath 'Local\\Pastral\.NativeManager\.Build' 'shared native XAML build mutex'
    Assert-Contains $PSCommandPath '/p:OutDir=' 'isolated native output override'
    Assert-Contains $PSCommandPath '/p:IntDir=' 'isolated native intermediate override'
    Assert-Contains $PSCommandPath 'target\\verification\\pastral-manager-(ipc|live)-' 'per-run native verification root'
    Assert-Contains $PSCommandPath 'Assert-ExactBridgeExports\s+-Path\s+\$bridgeDllSource' 'exact native bridge export gate'
    Assert-Contains $PSCommandPath '__Disallow_Upb_And_Cpp_In_Same_Binary' 'required Protobuf upb binary guard export'

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

    $tree = @(& cargo tree --locked -p pastral-manager-ipc-bridge --edges normal,build --prefix none --format '{p}')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $treeText = $tree -join "`n"
    foreach ($forbidden in @('tokio ', 'reqwest ', 'hyper ', 'serde_json ', 'rusqlite ')) {
        if ($treeText.Contains($forbidden)) {
            Fail "Manager bridge dependency tree contains forbidden package: $forbidden"
        }
    }

    Write-Host 'Manager IPC bridge Rust tests, build, and dependency policy: PASS'
}

function Invoke-ProbeVerification {
    Write-Host 'Building native manager IPC probe'
    $temporary = Join-Path $repositoryRoot ('target\verification\pastral-manager-ipc-' + [guid]::NewGuid().ToString('N'))
    $probeOutput = Join-Path $temporary 'probe-out'
    $probeIntermediate = Join-Path $temporary 'probe-obj'
    $dataRoot = Join-Path $temporary 'data'
    $stdout = Join-Path $temporary 'agent.out'
    $stderr = Join-Path $temporary 'agent.err'
    $readStdout = Join-Path $temporary 'agent-read.out'
    $readStderr = Join-Path $temporary 'agent-read.err'
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null

    $agent = $null
    try {
        & cargo build --locked -p pastral-manager-ipc-bridge --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Assert-ExactBridgeExports -Path $bridgeDllSource
        & cargo build --locked -p pastral-agent --features ipc-health --bin pastral-agent-ipc --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Invoke-MSBuildProject -Project $probeProject -Configuration Release `
            -OutputDirectory $probeOutput -IntermediateDirectory $probeIntermediate

        $probe = Join-Path $probeOutput 'Pastral.Manager.IpcProbe.exe'
        if (-not (Test-Path -LiteralPath $probe -PathType Leaf)) {
            $probe = Join-Path $probeOutput 'pastral-manager-ipc-probe.exe'
        }
        Assert-File $probe
        Copy-Item -LiteralPath $bridgeDllSource -Destination (Join-Path $probeOutput $bridgeDllName) -Force

        $abiOutput = @(& $probe --abi 2>&1)
        $abiText = $abiOutput -join "`n"
        if ($LASTEXITCODE -ne 0 -or
            -not $abiText.Contains('manager-ipc-abi=ok') -or
            -not $abiText.Contains('manager-ipc-read-abi=ok')) {
            Fail "Native bridge ABI probe failed: $($abiOutput -join ' ')"
        }

        $agent = Start-AgentServer -Command serve-health -DataRoot $dataRoot -MaxConnections 1 -OutputPath $stdout -ErrorPath $stderr
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
        $agent.Dispose()
        $agent = $null

        $agent = Start-AgentServer -Command serve-read -DataRoot $dataRoot -MaxConnections 2 -OutputPath $readStdout -ErrorPath $readStderr
        $readOutput = @(& $probe --read --data-root $dataRoot 2>&1)
        $readExit = $LASTEXITCODE
        if ($readExit -ne 0) {
            Fail "Native read probe failed with code ${readExit}: $($readOutput -join ' ')"
        }
        $readText = $readOutput -join "`n"
        foreach ($marker in @(
            'manager-ipc-read-probe=ok',
            'history-status=0',
            'history-count=0',
            'history-has-more=0',
            'search-status=0',
            'search-count=0',
            'search-has-more=0'
        )) {
            if (-not $readText.Contains($marker)) {
                Fail "Native read probe output is missing: $marker"
            }
        }
        foreach ($forbidden in @('secret=', 'nonce=', 'proof=', '\\.\pipe\', $dataRoot.ToLowerInvariant())) {
            if ($readText.ToLowerInvariant().Contains($forbidden.ToLowerInvariant())) {
                Fail "Native read probe emitted forbidden content: $forbidden"
            }
        }
        if (-not $agent.WaitForExit(10000)) {
            Fail 'Agent read server did not exit after the bounded probe connections'
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
    $runtime = @(Get-AppxPackage -Name 'Microsoft.WindowsAppRuntime.2' -ErrorAction SilentlyContinue |
        Where-Object { $_.Architecture -eq 'X64' -and $_.Version -eq [version]'2.3.1.0' })
    if ($runtime.Count -eq 0) {
        Fail 'Microsoft.WindowsAppRuntime.2 x64 version 2.3.1.0 is required for live manager verification'
    }

    $temporary = Join-Path $repositoryRoot ('target\verification\pastral-manager-live-' + [guid]::NewGuid().ToString('N'))
    $managerOutput = Join-Path $temporary 'manager-out'
    $managerIntermediate = Join-Path $temporary 'manager-obj'
    $dataRoot = Join-Path $temporary 'data'
    $stdout = Join-Path $temporary 'agent.out'
    $stderr = Join-Path $temporary 'agent.err'
    New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null

    $agent = $null
    $managerProcess = $null
    $oldDiagnostic = $env:PASTRAL_MANAGER_DIAGNOSTIC
    $oldRoot = $env:PASTRAL_MANAGER_DATA_ROOT
    try {
        & cargo build --locked -p pastral-agent --features ipc-health --bin pastral-agent-ipc --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Invoke-MSBuildProject -Project $managerProject -Configuration Release `
            -OutputDirectory $managerOutput -IntermediateDirectory $managerIntermediate

        $manager = Join-Path $managerOutput 'pastral-manager.exe'
        Assert-File $manager
        Assert-File (Join-Path $managerOutput $bridgeDllName)

        $agent = Start-AgentServer -Command serve-read -DataRoot $dataRoot -MaxConnections 6 -OutputPath $stdout -ErrorPath $stderr
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

        $history = Find-AutomationElementByName -Root $root -Name 'History' -TimeoutSeconds 5
        $historySelection = $history.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
        $historySelection.Select()
        $search = Find-AutomationElementByName -Root $root -Name 'Search clipboard history' -TimeoutSeconds 10
        $editCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit
        )
        $edit = $search.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $editCondition)
        if ($null -eq $edit) {
            $edit = $root.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $editCondition)
        }
        if ($null -eq $edit) {
            Fail 'History Search edit control is missing from the UI Automation tree'
        }
        $value = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        $value.SetValue('probe')
        [void](Find-AutomationElementByName -Root $root -Name 'No matching clips' -TimeoutSeconds 10)
        if (-not $agent.WaitForExit(15000)) {
            Fail 'Agent read server did not exit after Home, History, and Search connections'
        }

        $overview = Find-AutomationElementByName -Root $root -Name 'Overview' -TimeoutSeconds 5
        $overviewSelection = $overview.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
        $overviewSelection.Select()
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

    Write-Host 'Release manager live History/Search and Connected-to-Disconnected UIA smoke: PASS'
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
