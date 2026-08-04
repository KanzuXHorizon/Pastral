[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Build', 'Smoke', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$agentRoot = Join-Path $repositoryRoot 'apps\agent'

function Fail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Error $Message
    exit 1
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
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )) {
        Fail "$Description is missing from $Path"
    }
}

function Invoke-StaticVerification {
    Write-Host 'Pastral agent static verification'
    $required = @(
        'Cargo.toml',
        'src\main.rs',
        'src\cli.rs',
        'src\runtime.rs',
        'src\platform.rs',
        'src\privacy_config.rs',
        'src\storage_sink.rs',
        'src\config.rs'
    )
    $missing = @(
        $required | Where-Object {
            -not (Test-Path -LiteralPath (Join-Path $agentRoot $_) -PathType Leaf)
        }
    )
    if ($missing.Count -gt 0) {
        Fail ('Missing agent files: ' + ($missing -join ', '))
    }

    $cli = Join-Path $agentRoot 'src\cli.rs'
    Assert-Contains $cli '"health-check"' 'health-check command'
    Assert-Contains $cli '"capture-current"' 'capture-current command'
    Assert-Contains $cli '"listen"' 'listen command'
    Assert-Contains $cli 'MissingCommand' 'missing-command fail-closed state'
    Assert-Contains $cli 'MissingDataRoot' 'required data-root state'
    Assert-Contains $cli 'InvalidMaxEvents' 'bounded listen state'

    $main = Join-Path $agentRoot 'src\main.rs'
    Assert-Contains $main 'args_os\(\)\.skip\(1\)' 'explicit command parsing'
    Assert-Contains $main 'parse_arguments' 'fail-closed CLI parser use'
    Assert-Contains $main 'ExitCode::from\(2\)' 'invalid CLI nonzero exit'

    $runtime = Join-Path $agentRoot 'src\runtime.rs'
    Assert-Contains $runtime 'MAX_UNICODE_TEXT_BYTES:\s*usize\s*=\s*16\s*\*\s*1024\s*\*\s*1024' '16 MiB capture limit'
    Assert-Contains $runtime 'Duration::from_millis\(5\)' '5 ms retry delay'
    Assert-Contains $runtime 'Duration::from_millis\(15\)' '15 ms retry delay'
    Assert-Contains $runtime 'Duration::from_millis\(35\)' '35 ms retry delay'
    Assert-Contains $runtime 'integrity_check\(\)' 'storage integrity health check'
    Assert-Contains $runtime 'PrivacyPolicyConfig::load_or_create' 'strict privacy policy loading'
    Assert-Contains $runtime 'privacy-policy=ok' 'privacy policy health marker'
    Assert-Contains $runtime 'capture-outcome=stored' 'content-free stored outcome'
    Assert-Contains $runtime 'capture-outcome=hard-denied' 'hard-deny outcome'
    Assert-Contains $runtime 'capture-outcome=policy-denied' 'source-policy outcome'
    Assert-Contains $runtime 'capture-outcome=sensitive-skipped' 'sensitive-skip outcome'

    $privacyConfig = Join-Path $agentRoot 'src\privacy_config.rs'
    Assert-Contains $privacyConfig 'PRIVACY_POLICY_FILE:\s*&str\s*=\s*"privacy-policy\.txt"' 'privacy policy filename'
    Assert-Contains $privacyConfig 'SourceAdmissionPolicy::new\(\s*true' 'fail-closed unresolved-source default'
    foreach ($executable in @('1password\.exe', 'bitwarden\.exe', 'keepass\.exe', 'keepassxc\.exe')) {
        Assert-Contains $privacyConfig $executable 'baseline denied executable'
    }

    $privacyCore = Join-Path $repositoryRoot 'crates\agent-core\src\privacy.rs'
    Assert-Contains $privacyCore 'MAX_SECRET_SCAN_BYTES:\s*usize\s*=\s*1024\s*\*\s*1024' '1 MiB detector bound'
    Assert-Contains $privacyCore 'PrivateKeyMaterial' 'private-key sensitive class'
    Assert-Contains $privacyCore 'DetectorLimitExceeded' 'detector-limit sensitive class'

    $historyControls = Join-Path $repositoryRoot 'crates\clipboard-win\src\history_controls.rs'
    Assert-Contains $historyControls 'ExcludeClipboardContentFromMonitorProcessing' 'source-owned monitor exclusion format'
    Assert-Contains $historyControls 'CanIncludeInClipboardHistory' 'source-owned history inclusion format'
    Assert-Contains $historyControls 'CanUploadToCloudClipboard' 'cloud-only clipboard control format'

    $runtimeContent = [System.IO.File]::ReadAllText($runtime)
    foreach ($pattern in @(
        '(?i)clipboard-text',
        '(?i)content-hash',
        '(?i)raw_utf16le\s*\(',
        '(?i)captured_text\s*\(\)\.text\s*\('
    )) {
        if ([System.Text.RegularExpressions.Regex]::IsMatch($runtimeContent, $pattern)) {
            Fail "Content-bearing output pattern '$pattern' found in runtime"
        }
    }

    $scriptContent = [System.IO.File]::ReadAllText($PSCommandPath)
    foreach ($forbiddenInvocation in @(
        "& `$executable 'capture-current'",
        "& `$executable 'listen'",
        "Start-Process -FilePath `$executable -ArgumentList 'capture-current'",
        "Start-Process -FilePath `$executable -ArgumentList 'listen'"
    )) {
        if ($scriptContent.Contains($forbiddenInvocation)) {
            Fail 'Automated agent verification must not invoke clipboard-reading commands'
        }
    }

    Write-Host 'Agent static policy: PASS'
}

function Invoke-BuildVerification {
    Write-Host 'Building pastral-agent Debug'
    & cargo build --locked -p pastral-agent
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
    Write-Host 'Building pastral-agent Release'
    & cargo build --locked -p pastral-agent --release
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
    Write-Host 'Agent Debug and Release builds: PASS'
}

function Resolve-DebugExecutable {
    $candidate = Join-Path $repositoryRoot 'target\debug\pastral-agent.exe'
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        Fail "Debug agent executable was not found at $candidate"
    }
    return $candidate
}

function Invoke-SmokeVerification {
    $executable = Resolve-DebugExecutable
    $temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("pastral-agent-smoke-" + [guid]::NewGuid().ToString('N'))
    try {
        New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null
        $output = @(& $executable 'health-check' '--data-root' $temporaryRoot 2>&1)
        $exitCode = $LASTEXITCODE
        $text = ($output -join "`n")
        if ($exitCode -ne 0) {
            Fail "Agent health-check exited with code ${exitCode}: $text"
        }
        foreach ($marker in @(
            'agent-health=ok',
            'privacy-policy=ok',
            'storage-schema=1',
            'sqlite-integrity=ok',
            'fts-integrity=ok',
            'metadata-integrity=ok',
            'search-mapping-integrity=ok'
        )) {
            if (-not $text.Contains($marker)) {
                Fail "Agent health-check output is missing marker: $marker"
            }
        }
        foreach ($forbidden in @('clipboard-text', 'content-hash')) {
            if ($text.ToLowerInvariant().Contains($forbidden)) {
                Fail "Agent health-check emitted forbidden content marker: $forbidden"
            }
        }
        if (-not (Test-Path -LiteralPath (Join-Path $temporaryRoot 'agent-identity.txt') -PathType Leaf)) {
            Fail 'Agent health-check did not create its content-free identity file'
        }
        if (-not (Test-Path -LiteralPath (Join-Path $temporaryRoot 'privacy-policy.txt') -PathType Leaf)) {
            Fail 'Agent health-check did not create its strict privacy policy file'
        }
        if (-not (Test-Path -LiteralPath (Join-Path $temporaryRoot 'storage\metadata.sqlite3') -PathType Leaf)) {
            Fail 'Agent health-check did not create/open storage metadata'
        }

        $previousErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = 'Continue'
            & $executable *> $null
            $invalidCommandExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        if ($invalidCommandExitCode -eq 0) {
            Fail 'Agent without an explicit command must fail closed'
        }
        $global:LASTEXITCODE = 0
    }
    finally {
        if (Test-Path -LiteralPath $temporaryRoot) {
            Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    Write-Host 'Agent health-check smoke: PASS'
}

Push-Location $repositoryRoot
try {
    switch ($Mode) {
        'Static' { Invoke-StaticVerification }
        'Build' { Invoke-BuildVerification }
        'Smoke' { Invoke-SmokeVerification }
        'All' {
            Invoke-StaticVerification
            Invoke-BuildVerification
            Invoke-SmokeVerification
        }
    }
}
finally {
    Pop-Location
}
