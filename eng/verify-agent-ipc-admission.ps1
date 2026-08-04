[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Test', 'Smoke', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$probeRoot = Join-Path $repositoryRoot 'apps\agent-ipc-probe'
$agentManifest = Join-Path $repositoryRoot 'apps\agent\Cargo.toml'
$probeManifest = Join-Path $probeRoot 'Cargo.toml'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Error $Message
    exit 1
}

function Assert-File {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Required agent IPC admission file is missing: $Path"
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

function Parse-Metrics {
    param([Parameter(Mandatory = $true)][string[]]$Lines)
    $metrics = @{}
    foreach ($line in $Lines) {
        $separator = $line.IndexOf('=')
        if ($separator -gt 0) {
            $metrics[$line.Substring(0, $separator)] = $line.Substring($separator + 1)
        }
    }
    return $metrics
}

function Require-Metric {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Metrics,
        [Parameter(Mandatory = $true)][string]$Name
    )
    if (-not $Metrics.ContainsKey($Name)) {
        Fail "Agent IPC admission output is missing metric: $Name"
    }
    return $Metrics[$Name]
}

function Invoke-StaticVerification {
    Write-Host 'Pastral agent IPC admission static verification'
    foreach ($path in @(
        $probeManifest,
        (Join-Path $probeRoot 'src\main.rs'),
        (Join-Path $probeRoot 'src\cli.rs'),
        (Join-Path $probeRoot 'src\child.rs'),
        (Join-Path $probeRoot 'src\server.rs'),
        (Join-Path $probeRoot 'src\parent.rs'),
        (Join-Path $probeRoot 'src\metrics.rs'),
        (Join-Path $probeRoot 'src\protocol.rs'),
        (Join-Path $probeRoot 'tests\cross_process.rs'),
        (Join-Path $repositoryRoot 'apps\agent\src\health.rs'),
        (Join-Path $repositoryRoot 'apps\agent\src\ipc_health.rs'),
        (Join-Path $repositoryRoot 'crates\ipc-win\src\process_memory.rs')
    )) {
        Assert-File $path
    }

    $agentTree = @(& cargo tree --locked -p pastral-agent --edges all --prefix none --format '{p}')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $agentTreeText = $agentTree -join "`n"
    foreach ($forbidden in @(
        '(?m)^protobuf\s',
        '(?m)^protobuf-codegen\s',
        '(?m)^protobuf-macros\s',
        '(?m)^pastral-ipc-auth\s',
        '(?m)^pastral-ipc-schema\s',
        '(?m)^pastral-ipc-win\s'
    )) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($agentTreeText, $forbidden)) {
            Fail "Default agent dependency graph contains forbidden IPC/runtime package: $forbidden"
        }
    }

    $manifest = [System.IO.File]::ReadAllText($probeManifest)
    foreach ($required in @(
        'pastral-agent\s*=\s*\{\s*path',
        'pastral-ipc-auth\s*=\s*\{\s*path',
        'pastral-ipc-core\s*=\s*\{\s*path',
        'pastral-ipc-schema\s*=\s*\{\s*path',
        'pastral-ipc-win\s*=\s*\{\s*path'
    )) {
        if (-not [System.Text.RegularExpressions.Regex]::IsMatch($manifest, $required)) {
            Fail "Admission manifest is missing required dependency: $required"
        }
    }
    foreach ($forbidden in @('pastral-storage\s*=', 'pastral-clipboard-win\s*=', 'tokio\s*=', 'reqwest\s*=', 'tracing\s*=', 'log\s*=')) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($manifest, $forbidden)) {
            Fail "Admission manifest contains forbidden direct dependency: $forbidden"
        }
    }

    $cli = Join-Path $probeRoot 'src\cli.rs'
    Assert-Contains $cli 'AdmissionMode::Parent' 'parent admission mode'
    Assert-Contains $cli '"--baseline-child"' 'baseline child mode'
    Assert-Contains $cli '"--server-child"' 'server child mode'
    Assert-Contains $cli '"--data-root"' 'required child data root'

    $child = Join-Path $probeRoot 'src\child.rs'
    Assert-Contains $child 'load_health_snapshot' 'real agent Health snapshot use'
    Assert-Contains $child 'agent-baseline-ready=ok' 'baseline readiness marker'

    $server = Join-Path $probeRoot 'src\server.rs'
    Assert-Contains $server 'serve_health' 'shared agent Health server delegation'

    $sharedServer = Join-Path $repositoryRoot 'apps\agent\src\ipc_health.rs'
    Assert-Contains $sharedServer 'agent-ipc-ready=1' 'server readiness marker'
    Assert-Contains $sharedServer 'server_handshake' 'authenticated server handshake'
    Assert-Contains $sharedServer 'RequestDto::Health' 'Health-only request authorization'
    Assert-Contains $sharedServer 'load_health_snapshot\(data_root\)' 'per-request Health reload'

    $metrics = Join-Path $probeRoot 'src\metrics.rs'
    Assert-Contains $metrics '25\s*\*\s*MIB' '25 MiB server private ceiling'
    Assert-Contains $metrics '8\s*\*\s*MIB' '8 MiB private delta ceiling'
    Assert-Contains $metrics '12\s*\*\s*MIB' '12 MiB working-set delta ceiling'
    Assert-Contains $metrics '6\s*\*\s*MIB' '6 MiB binary delta ceiling'

    $parent = Join-Path $probeRoot 'src\parent.rs'
    foreach ($marker in @(
        'agent-ipc-admission=ok',
        'cross-process=true',
        'health=ok',
        'default-agent-binary-bytes=',
        'baseline-private-bytes=',
        'server-private-bytes=',
        'private-delta-bytes='
    )) {
        Assert-Contains $parent ([System.Text.RegularExpressions.Regex]::Escape($marker)) "output marker $marker"
    }
    foreach ($forbidden in @('capture-current', 'AgentCommand::Listen', 'AgentCommand::CaptureCurrent', 'ClipboardSession', 'localhost', 'AppData')) {
        if ([System.IO.File]::ReadAllText($parent).Contains($forbidden)) {
            Fail "Admission parent contains forbidden behavior marker: $forbidden"
        }
    }

    Write-Host 'Agent IPC admission static policy: PASS'
}

function Invoke-TestVerification {
    Write-Host 'Testing shared agent Health snapshot'
    & cargo test --locked -p pastral-agent --test health_snapshot --test runtime
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing process memory evidence'
    & cargo test --locked -p pastral-ipc-win --test process_memory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing agent IPC admission package'
    & cargo test --locked -p pastral-agent-ipc-probe --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Agent IPC admission tests: PASS'
}

function Invoke-SmokeVerification {
    Write-Host 'Building default agent Release'
    & cargo build --locked -p pastral-agent --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Building agent IPC admission Release'
    & cargo build --locked -p pastral-agent-ipc-probe --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $executable = Join-Path $repositoryRoot 'target\release\pastral-agent-ipc-probe.exe'
    Assert-File $executable
    $output = @(& $executable 2>&1)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        Fail "Agent IPC admission probe exited with code ${exitCode}: $($output -join ' ')"
    }
    $metrics = Parse-Metrics -Lines $output
    if ((Require-Metric $metrics 'agent-ipc-admission') -ne 'ok') { Fail 'Admission did not report success' }
    if ((Require-Metric $metrics 'cross-process') -ne 'true') { Fail 'Admission was not cross-process' }
    if ((Require-Metric $metrics 'health') -ne 'ok') { Fail 'Admission Health failed' }
    if ((Require-Metric $metrics 'admission-ceilings') -ne 'passed') { Fail 'Release admission ceilings did not pass' }

    $clientPid = [uint32](Require-Metric $metrics 'client-pid')
    $serverPid = [uint32](Require-Metric $metrics 'server-pid')
    if ($clientPid -eq 0 -or $serverPid -eq 0 -or $clientPid -eq $serverPid) {
        Fail 'Admission PID evidence is invalid'
    }

    $defaultBinary = [uint64](Require-Metric $metrics 'default-agent-binary-bytes')
    $admissionBinary = [uint64](Require-Metric $metrics 'admission-binary-bytes')
    $binaryDelta = [uint64](Require-Metric $metrics 'binary-delta-bytes')
    $baselineWorking = [uint64](Require-Metric $metrics 'baseline-working-set-bytes')
    $baselinePrivate = [uint64](Require-Metric $metrics 'baseline-private-bytes')
    $serverWorking = [uint64](Require-Metric $metrics 'server-working-set-bytes')
    $serverPrivate = [uint64](Require-Metric $metrics 'server-private-bytes')
    $workingDelta = [int64](Require-Metric $metrics 'working-set-delta-bytes')
    $privateDelta = [int64](Require-Metric $metrics 'private-delta-bytes')

    foreach ($value in @($defaultBinary, $admissionBinary, $baselineWorking, $baselinePrivate, $serverWorking, $serverPrivate)) {
        if ($value -eq 0) { Fail 'Admission emitted zero byte metric' }
    }
    if ($admissionBinary -lt $defaultBinary -or $admissionBinary - $defaultBinary -ne $binaryDelta) {
        Fail 'Admission binary delta is inconsistent'
    }
    if ($binaryDelta -gt 6MB) { Fail 'Admission binary delta exceeds 6 MiB' }
    if ($serverPrivate -gt 25MB) { Fail 'Admission server private usage exceeds 25 MiB' }
    if ($workingDelta -gt 12MB) { Fail 'Admission working-set delta exceeds 12 MiB' }
    if ($privateDelta -gt 8MB) { Fail 'Admission private delta exceeds 8 MiB' }

    foreach ($name in @('connect-us', 'handshake-us', 'health-us', 'total-us')) {
        if ([uint64](Require-Metric $metrics $name) -eq 0) {
            Fail "Admission timing metric is zero: $name"
        }
    }

    $text = ($output -join "`n").ToLowerInvariant()
    foreach ($forbidden in @('\\.\pipe\', 'secret=', 'nonce=', 'proof=', 'root=', 'sid=', 'clipboard', 'preview=', 'query=')) {
        if ($text.Contains($forbidden)) {
            Fail "Admission emitted forbidden marker: $forbidden"
        }
    }

    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $invalidOutput = @(& $executable '--unknown' 2>&1)
        $invalidExit = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($invalidExit -ne 2) { Fail "Invalid admission arguments returned $invalidExit instead of 2" }
    if (($invalidOutput -join "`n") -ne 'agent-ipc-admission=invalid arguments') {
        Fail 'Invalid admission arguments emitted unexpected output'
    }

    $output | ForEach-Object { Write-Host $_ }
    $global:LASTEXITCODE = 0
    Write-Host 'Agent IPC admission Release smoke: PASS'
}

Push-Location $repositoryRoot
try {
    switch ($Mode) {
        'Static' { Invoke-StaticVerification }
        'Test' { Invoke-TestVerification }
        'Smoke' { Invoke-SmokeVerification }
        'All' {
            Invoke-StaticVerification
            Invoke-TestVerification
            Invoke-SmokeVerification
        }
    }
}
finally {
    Pop-Location
}
