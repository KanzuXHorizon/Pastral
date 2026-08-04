[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Test', 'Probe', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$protoPath = Join-Path $repositoryRoot 'protocols\ipc-schema\pastral_ipc_v1.proto'
$expectedSchemaSha256 = '2029ac9b19f7eb1644a2c12b3cd570586af9b62c40e130558b63c376676e3077'
$ipcCoreRoot = Join-Path $repositoryRoot 'crates\ipc-core'
$ipcSchemaRoot = Join-Path $repositoryRoot 'crates\ipc-schema'
$probeRoot = Join-Path $repositoryRoot 'apps\ipc-probe'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Error $Message
    exit 1
}

function Assert-File {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Required IPC prototype file is missing: $Path"
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

function Resolve-Protoc {
    if ($env:PROTOC -and (Test-Path -LiteralPath $env:PROTOC -PathType Leaf)) {
        return $env:PROTOC
    }
    $command = Get-Command protoc.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    $packages = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages'
    if (Test-Path -LiteralPath $packages -PathType Container) {
        $candidate = Get-ChildItem -LiteralPath $packages -Directory -Filter 'Google.Protobuf_*' |
            ForEach-Object { Join-Path $_.FullName 'bin\protoc.exe' } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
        if ($candidate) {
            return $candidate
        }
    }
    Fail 'Exact protoc 35.0 is required. Install Google.Protobuf 35.0 or set PROTOC.'
}

function Invoke-StaticVerification {
    Write-Host 'Pastral IPC prototype static verification'
    foreach ($path in @(
        $protoPath,
        (Join-Path $ipcCoreRoot 'Cargo.toml'),
        (Join-Path $ipcCoreRoot 'src\frame.rs'),
        (Join-Path $ipcCoreRoot 'src\decoder.rs'),
        (Join-Path $ipcCoreRoot 'src\connection.rs'),
        (Join-Path $ipcCoreRoot 'src\dto.rs'),
        (Join-Path $ipcSchemaRoot 'Cargo.toml'),
        (Join-Path $ipcSchemaRoot 'build.rs'),
        (Join-Path $ipcSchemaRoot 'src\convert.rs'),
        (Join-Path $probeRoot 'Cargo.toml'),
        (Join-Path $probeRoot 'src\main.rs')
    )) {
        Assert-File $path
    }

    Assert-Contains $protoPath '^\s*edition\s*=\s*"2024"\s*;' 'Edition 2024 schema authority'
    Assert-Contains $protoPath '^\s*package\s+pastral\.ipc\.v1\s*;' 'versioned IPC package'
    Assert-Contains $protoPath 'oneof\s+operation' 'bounded request/response oneof'

    $proto = [System.IO.File]::ReadAllText($protoPath)
    foreach ($forbidden in @(
        '(?im)^\s*service\s+',
        '(?i)google\.protobuf\.Any',
        '(?i)\bmap\s*<',
        '(?im)^\s*import\s+',
        '(?i)clipboard_payload',
        '(?i)file_path',
        '(?i)window_handle'
    )) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($proto, $forbidden)) {
            Fail "Forbidden IPC schema pattern found: $forbidden"
        }
    }

    $workspaceManifest = Join-Path $repositoryRoot 'Cargo.toml'
    Assert-Contains $workspaceManifest 'protobuf\s*=\s*\{\s*version\s*=\s*"=4\.35\.0-release"' 'exact protobuf runtime pin'
    Assert-Contains $workspaceManifest 'protobuf-codegen\s*=\s*\{\s*version\s*=\s*"=4\.35\.0-release"' 'exact protobuf codegen pin'
    Assert-Contains (Join-Path $ipcCoreRoot 'src\frame.rs') 'FRAME_HEADER_BYTES:\s*usize\s*=\s*36' 'exact 36-byte frame header'
    Assert-Contains (Join-Path $ipcCoreRoot 'src\lib.rs') '#!\[forbid\(unsafe_code\)\]' 'safe IPC core boundary'
    Assert-Contains (Join-Path $ipcSchemaRoot 'src\lib.rs') 'PROTOBUF_RELEASE:\s*&str\s*=\s*"4\.35\.0-release"' 'runtime release marker'
    Assert-Contains (Join-Path $ipcSchemaRoot 'build.rs') 'protoc_path\(' 'explicit protoc path control'

    $schemaSource = @(
        Get-ChildItem -LiteralPath $ipcSchemaRoot -Recurse -File -Include '*.rs','Cargo.toml'
    )
    $probeSource = @(
        Get-ChildItem -LiteralPath $probeRoot -Recurse -File -Include '*.rs','Cargo.toml'
    )
    $content = (($schemaSource + $probeSource) | ForEach-Object {
        [System.IO.File]::ReadAllText($_.FullName)
    }) -join "`n"
    foreach ($forbidden in @(
        '(?i)\btokio\b',
        '(?i)\btonic\b',
        '(?i)\bprost\b',
        '(?i)\bserde(_json)?\b',
        '(?i)\breqwest\b',
        '(?i)\bhyper\b',
        '(?i)std::net',
        '(?i)Tcp(Stream|Listener)',
        '(?i)UdpSocket',
        '(?i)ClipboardSession',
        '(?i)pastral_storage',
        '(?i)CreateNamedPipe',
        '(?i)ConnectNamedPipe'
    )) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($content, $forbidden)) {
            Fail "Forbidden IPC prototype dependency/API pattern found: $forbidden"
        }
    }

    $protoc = Resolve-Protoc
    $version = (& $protoc --version).Trim()
    if ($LASTEXITCODE -ne 0 -or $version -ne 'libprotoc 35.0') {
        Fail "Expected libprotoc 35.0, found '$version' at $protoc"
    }
    Write-Host "protoc: $version"
    Write-Host "protoc path: $protoc"
    Write-Host 'IPC prototype static policy: PASS'
}

function Invoke-TestVerification {
    Write-Host 'Testing IPC framing core'
    & cargo test --locked -p pastral-ipc-core --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing IPC Edition 2024 schema'
    & cargo test --locked -p pastral-ipc-schema --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'Testing IPC release probe'
    & cargo test --locked -p pastral-ipc-probe --all-targets
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host 'IPC prototype tests: PASS'
}

function Parse-Metrics {
    param([Parameter(Mandatory = $true)][string[]]$Lines)
    $metrics = @{}
    foreach ($line in $Lines) {
        $separator = $line.IndexOf('=')
        if ($separator -le 0) {
            continue
        }
        $metrics[$line.Substring(0, $separator)] = $line.Substring($separator + 1)
    }
    return $metrics
}

function Require-Metric {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Metrics,
        [Parameter(Mandatory = $true)][string]$Name
    )
    if (-not $Metrics.ContainsKey($Name)) {
        Fail "IPC probe output is missing metric: $Name"
    }
    return $Metrics[$Name]
}

function Invoke-ProbeVerification {
    Write-Host 'Building IPC probe Release'
    & cargo build --locked -p pastral-ipc-probe --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $executable = Join-Path $repositoryRoot 'target\release\pastral-ipc-probe.exe'
    Assert-File $executable
    $output = @(& $executable --iterations 10000 2>&1)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        Fail "IPC probe exited with code ${exitCode}: $($output -join "`n")"
    }
    $metrics = Parse-Metrics -Lines $output
    if ((Require-Metric $metrics 'ipc-probe') -ne 'ok') {
        Fail 'IPC probe did not report success'
    }
    if ((Require-Metric $metrics 'protobuf-release') -ne '4.35.0-release') {
        Fail 'IPC probe used an unexpected protobuf runtime release'
    }
    $schemaDigest = Require-Metric $metrics 'schema-sha256'
    if ($schemaDigest -notmatch '^[0-9a-f]{64}$') {
        Fail 'IPC probe schema digest is not lowercase SHA-256'
    }
    if ($schemaDigest -ne $expectedSchemaSha256) {
        Fail "IPC probe schema digest drifted: expected $expectedSchemaSha256, received $schemaDigest"
    }
    if ([uint32](Require-Metric $metrics 'iterations') -ne 10000) {
        Fail 'IPC probe iteration count is incorrect'
    }
    if ([uint32](Require-Metric $metrics 'round-trips') -ne 10000) {
        Fail 'IPC probe did not complete all round trips'
    }
    if ([uint64](Require-Metric $metrics 'executable-bytes') -eq 0) {
        Fail 'IPC probe executable size is zero'
    }
    $averageRoundTrip = [uint64](Require-Metric $metrics 'average-roundtrip-ns')
    if ($averageRoundTrip -ge 1000000) {
        Fail "IPC probe average round trip exceeded 1 ms smoke ceiling: $averageRoundTrip ns"
    }
    $maxCapacity = [uint64](Require-Metric $metrics 'max-body-capacity')
    if ($maxCapacity -eq 0 -or $maxCapacity -gt (256 * 1024)) {
        Fail "IPC probe body capacity exceeded control bound: $maxCapacity"
    }
    foreach ($forbidden in @('synthetic-preview', 'synthetic-source', 'clipboard-text', 'search-query')) {
        if (($output -join "`n").ToLowerInvariant().Contains($forbidden)) {
            Fail "IPC probe emitted forbidden content marker: $forbidden"
        }
    }

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $executable --unknown *> $null
        $invalidExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($invalidExitCode -eq 0) {
        Fail 'IPC probe unknown arguments must fail closed'
    }
    $global:LASTEXITCODE = 0

    $output | ForEach-Object { Write-Host $_ }
    Write-Host 'IPC prototype release probe: PASS'
}

Push-Location $repositoryRoot
try {
    switch ($Mode) {
        'Static' { Invoke-StaticVerification }
        'Test' { Invoke-TestVerification }
        'Probe' { Invoke-ProbeVerification }
        'All' {
            Invoke-StaticVerification
            Invoke-TestVerification
            Invoke-ProbeVerification
        }
    }
}
finally {
    Pop-Location
}
