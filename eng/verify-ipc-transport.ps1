[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Test', 'Smoke', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$ipcWinRoot = Join-Path $repositoryRoot 'crates\ipc-win'
$authRoot = Join-Path $repositoryRoot 'crates\ipc-auth'
$probeRoot = Join-Path $repositoryRoot 'apps\ipc-transport-probe'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Error $Message
    exit 1
}

function Assert-File {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Required IPC transport file is missing: $Path"
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
        Fail "IPC transport output is missing metric: $Name"
    }
    return $Metrics[$Name]
}

function Invoke-StaticVerification {
    Write-Host 'Pastral authenticated IPC transport static verification'
    foreach ($path in @(
        (Join-Path $authRoot 'src\transcript.rs'),
        (Join-Path $authRoot 'src\replay.rs'),
        (Join-Path $ipcWinRoot 'src\config.rs'),
        (Join-Path $ipcWinRoot 'src\dpapi.rs'),
        (Join-Path $ipcWinRoot 'src\token.rs'),
        (Join-Path $ipcWinRoot 'src\security.rs'),
        (Join-Path $ipcWinRoot 'src\pipe.rs'),
        (Join-Path $ipcWinRoot 'src\overlapped.rs'),
        (Join-Path $ipcWinRoot 'src\stream.rs'),
        (Join-Path $ipcWinRoot 'src\handshake.rs'),
        (Join-Path $ipcWinRoot 'src\sys.rs'),
        (Join-Path $probeRoot 'src\main.rs'),
        (Join-Path $probeRoot 'tests\cross_process.rs')
    )) {
        Assert-File $path
    }

    $sys = Join-Path $ipcWinRoot 'src\sys.rs'
    $security = Join-Path $ipcWinRoot 'src\security.rs'
    $handshake = Join-Path $ipcWinRoot 'src\handshake.rs'
    $probe = Join-Path $probeRoot 'src\main.rs'
    Assert-Contains $sys 'FILE_FLAG_FIRST_PIPE_INSTANCE' 'first-instance anti-squatting flag'
    Assert-Contains $sys 'PIPE_REJECT_REMOTE_CLIENTS' 'remote client rejection flag'
    Assert-Contains $sys 'PIPE_TYPE_BYTE' 'byte-mode pipe type'
    Assert-Contains $sys 'FILE_FLAG_OVERLAPPED' 'overlapped pipe I/O'
    Assert-Contains $sys 'SECURITY_IDENTIFICATION' 'identification-only client SQOS'
    Assert-Contains $sys 'GetNamedPipeClientProcessId' 'kernel client PID query'
    Assert-Contains $sys 'GetNamedPipeServerProcessId' 'kernel server PID query'
    Assert-Contains $sys 'CancelIoEx' 'exact overlapped cancellation'
    Assert-Contains $sys 'GetOverlappedResultEx' 'bounded overlapped wait'
    Assert-Contains $sys 'CryptProtectData' 'DPAPI secret protection'
    Assert-Contains $sys 'CRYPTPROTECT_UI_FORBIDDEN' 'noninteractive DPAPI policy'
    Assert-Contains $sys 'D:P\(A;;0x' 'protected DACL SDDL construction'
    Assert-Contains $sys 'PIPE_ACCESS_MASK' 'bounded pipe client access mask'
    Assert-Contains $security 'identity\.logon_sid\(\)' 'logon SID security principal'
    Assert-Contains $handshake 'verify_proof' 'mutual proof verification'
    Assert-Contains $handshake 'NonceReplayCache' 'handshake replay defense'
    Assert-Contains $probe 'Command::new' 'cross-process server child probe'

    $productContent = @(
        Get-ChildItem -LiteralPath $ipcWinRoot -Recurse -File -Include '*.rs','Cargo.toml'
        Get-ChildItem -LiteralPath $probeRoot -Recurse -File -Include '*.rs','Cargo.toml'
    ) | ForEach-Object { [System.IO.File]::ReadAllText($_.FullName) }
    $joined = $productContent -join "`n"
    foreach ($forbidden in @(
        '(?i)\bPIPE_NOWAIT\b',
        '(?i)\bPIPE_TYPE_MESSAGE\b',
        '(?i)CRYPTPROTECT_LOCAL_MACHINE',
        '(?i)\bstd::net\b',
        '(?i)\btokio\b',
        '(?i)\btonic\b',
        '(?i)\breqwest\b',
        '(?i)\bregistry\b',
        '(?i)ClipboardSession',
        '(?i)pastral_storage'
    )) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($joined, $forbidden)) {
            Fail "Forbidden IPC transport pattern found: $forbidden"
        }
    }
    Write-Host 'IPC transport static policy: PASS'
}

function Invoke-TestVerification {
    Write-Host 'Testing IPC authentication core'
    & cargo test --locked -p pastral-ipc-auth --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing Windows IPC transport'
    & cargo test --locked -p pastral-ipc-win --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing cross-process transport probe'
    & cargo test --locked -p pastral-ipc-transport-probe --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'IPC transport tests: PASS'
}

function Invoke-SmokeVerification {
    Write-Host 'Building authenticated IPC transport probe Release'
    & cargo build --locked -p pastral-ipc-transport-probe --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $executable = Join-Path $repositoryRoot 'target\release\pastral-ipc-transport-probe.exe'
    Assert-File $executable
    $output = @(& $executable 2>&1)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        Fail "IPC transport probe exited with code $exitCode"
    }
    $metrics = Parse-Metrics -Lines $output
    if ((Require-Metric $metrics 'ipc-transport-probe') -ne 'ok') {
        Fail 'IPC transport probe did not report success'
    }
    if ((Require-Metric $metrics 'cross-process') -ne 'true') {
        Fail 'IPC transport probe was not cross-process'
    }
    $clientPid = [uint32](Require-Metric $metrics 'client-pid')
    $serverPid = [uint32](Require-Metric $metrics 'server-pid')
    if ($clientPid -eq 0 -or $serverPid -eq 0 -or $clientPid -eq $serverPid) {
        Fail 'IPC transport probe PID evidence is invalid'
    }
    foreach ($name in @('connect-us', 'handshake-us', 'health-us', 'total-us')) {
        if ([uint64](Require-Metric $metrics $name) -eq 0) {
            Fail "IPC transport metric is zero: $name"
        }
    }
    $combined = ($output -join "`n").ToLowerInvariant()
    foreach ($forbidden in @('\\.\pipe\', 'secret=', 'nonce=', 'proof=', 'root=', 'sid=', 'clipboard')) {
        if ($combined.Contains($forbidden)) {
            Fail "IPC transport probe emitted forbidden marker: $forbidden"
        }
    }
    $output | ForEach-Object { Write-Host $_ }
    Write-Host 'Authenticated IPC transport smoke: PASS'
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
