[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
try {
    Write-Host 'Pastral source-policy verification'

    $trackedFiles = @(& git ls-files --cached --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $forbiddenTrackedPaths = @(
        'Start-DevSpace-MCP-Cloudflared.ps1'
    )
    $forbiddenTrackedExtensions = @('.pfx', '.p12', '.pem', '.key')

    $pathViolations = @(
        $trackedFiles | Where-Object {
            $path = $_
            ($forbiddenTrackedPaths -contains $path) -or
            $path.StartsWith('target/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $path.StartsWith('.vs/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $path -match '(?i)^apps/manager/.+/(x64|Generated Files|AppPackages)/' -or
            $path.StartsWith('.env', [System.StringComparison]::OrdinalIgnoreCase) -or
            ($forbiddenTrackedExtensions -contains [System.IO.Path]::GetExtension($path).ToLowerInvariant())
        }
    )
    if ($pathViolations.Count -gt 0) {
        Write-Error ('Forbidden tracked paths: ' + ($pathViolations -join ', '))
        exit 1
    }

    $textExtensions = @(
        '.rs', '.toml', '.ps1', '.yml', '.yaml', '.md', '.json', '.txt', '.props', '.targets',
        '.cpp', '.c', '.h', '.hpp', '.idl', '.proto', '.xaml', '.vcxproj', '.filters', '.resw', '.xml'
    )
    $secretPatterns = @(
        'AKIA[0-9A-Z]{16}',
        'ASIA[0-9A-Z]{16}',
        'gh[pousr]_[A-Za-z0-9_]{20,}',
        '-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----'
    )
    $unsafePattern = '(?m)\bunsafe\s+(fn|extern|impl|trait|\{)'
    $sourcePatterns = @(
        '(?m)\bstd::net\b',
        '(?m)\bload_extension\b',
        '(?m)\bATTACH\s+DATABASE\b',
        '(?m)journal_mode\s*=\s*WAL\b'
    )
    $processSpawnPatterns = @(
        '(?m)\bstd::process::Command\b',
        '(?m)\bCommand::new\s*\('
    )
    $namedPipePatterns = @(
        '(?m)\bCreateNamedPipe[AW]?\b',
        '(?m)\bConnectNamedPipe\b',
        '(?m)\bWaitNamedPipe[AW]?\b'
    )

    $violations = New-Object System.Collections.Generic.List[string]
    foreach ($relativePath in $trackedFiles) {
        $extension = [System.IO.Path]::GetExtension($relativePath).ToLowerInvariant()
        if ($textExtensions -notcontains $extension) {
            continue
        }
        $fullPath = Join-Path $repositoryRoot $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }
        $content = [System.IO.File]::ReadAllText($fullPath)
        foreach ($pattern in $secretPatterns) {
            if ([System.Text.RegularExpressions.Regex]::IsMatch($content, $pattern)) {
                $violations.Add("secret signature in $relativePath")
            }
        }
        $isRustProductSource =
            $relativePath.StartsWith('crates/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $relativePath.StartsWith('apps/agent/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $relativePath.StartsWith('apps/agent-ipc-probe/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $relativePath.StartsWith('apps/ipc-probe/', [System.StringComparison]::OrdinalIgnoreCase) -or
            $relativePath.StartsWith('apps/ipc-transport-probe/', [System.StringComparison]::OrdinalIgnoreCase)
        if ($isRustProductSource) {
            $isIpcWinSys = $relativePath.Equals(
                'crates/ipc-win/src/sys.rs',
                [System.StringComparison]::OrdinalIgnoreCase
            )
            $isReviewedUnsafeBoundary =
                $relativePath.Equals(
                    'crates/clipboard-win/src/sys.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'crates/manager-ipc-bridge/src/ffi.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'crates/manager-ipc-bridge/tests/ffi.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or $isIpcWinSys
            $unsafeMatch = [System.Text.RegularExpressions.Regex]::IsMatch(
                $content,
                $unsafePattern,
                [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
            )
            if ((-not $isReviewedUnsafeBoundary) -and $unsafeMatch) {
                $violations.Add("unsafe product-source pattern outside reviewed sys boundaries in $relativePath")
            }
            foreach ($pattern in $sourcePatterns) {
                if ([System.Text.RegularExpressions.Regex]::IsMatch(
                    $content,
                    $pattern,
                    [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
                )) {
                    $violations.Add("forbidden product-source pattern in $relativePath")
                }
            }
            $isReviewedProcessSpawnBoundary =
                $relativePath.Equals(
                    'apps/ipc-transport-probe/src/main.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'apps/ipc-transport-probe/tests/cross_process.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'crates/ipc-win/tests/process_memory.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'apps/agent-ipc-probe/src/parent.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -or
                $relativePath.Equals(
                    'apps/agent-ipc-probe/tests/cross_process.rs',
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            if (-not $isReviewedProcessSpawnBoundary) {
                foreach ($pattern in $processSpawnPatterns) {
                    if ([System.Text.RegularExpressions.Regex]::IsMatch(
                        $content,
                        $pattern,
                        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
                    )) {
                        $violations.Add("process spawning outside reviewed diagnostic boundaries in $relativePath")
                    }
                }
            }
            if (-not $isIpcWinSys) {
                foreach ($pattern in $namedPipePatterns) {
                    if ([System.Text.RegularExpressions.Regex]::IsMatch(
                        $content,
                        $pattern,
                        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
                    )) {
                        $violations.Add("named-pipe API outside ipc-win sys boundary in $relativePath")
                    }
                }
            }
        }
    }

    $managerNativeFiles = @(
        $trackedFiles | Where-Object {
            $_ -match '(?i)^apps/manager/Pastral\.Manager/.+\.(cpp|h|hpp)$'
        }
    )
    foreach ($relativePath in $managerNativeFiles) {
        $fullPath = Join-Path $repositoryRoot $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }
        $content = [System.IO.File]::ReadAllText($fullPath)
        $isReviewedLoader = $relativePath.Equals(
            'apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp',
            [System.StringComparison]::OrdinalIgnoreCase
        )
        if (-not $isReviewedLoader -and [System.Text.RegularExpressions.Regex]::IsMatch(
            $content,
            '(?m)\b(LoadLibraryExW|LoadLibraryW|LoadLibraryA|GetProcAddress)\b'
        )) {
            $violations.Add("native DLL loading outside reviewed manager bridge boundary in $relativePath")
        }
        if ([System.Text.RegularExpressions.Regex]::IsMatch(
            $content,
            '(?im)\b(sqlite3?_open|OpenClipboard|GetClipboardData|SetClipboardData|AddClipboardFormatListener|WinHttpOpen|InternetOpen[AW]?)\b'
        )) {
            $violations.Add("direct storage, clipboard, or network API in native manager source $relativePath")
        }
    }

    foreach ($relativePath in @(
        'crates/manager-ipc-bridge/src/lib.rs',
        'crates/manager-ipc-bridge/src/abi.rs',
        'crates/manager-ipc-bridge/src/client.rs',
        'crates/manager-ipc-bridge/src/ffi.rs'
    )) {
        $fullPath = Join-Path $repositoryRoot $relativePath
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }
        $content = [System.IO.File]::ReadAllText($fullPath)
        if ([System.Text.RegularExpressions.Regex]::IsMatch(
            $content,
            '(?im)\b(rusqlite|pastral_storage|pastral_clipboard|std::net|std::process::Command|Command::new)\b'
        )) {
            $violations.Add("forbidden storage, clipboard, network, or process behavior in $relativePath")
        }
    }

    if ($violations.Count -gt 0) {
        $violations | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
        exit 1
    }

    Write-Host 'Source policy: PASS'
}
finally {
    Pop-Location
}
