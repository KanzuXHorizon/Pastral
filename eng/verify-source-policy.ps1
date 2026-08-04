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
        '(?m)\bstd::process::Command\b',
        '(?m)\bCommand::new\s*\(',
        '(?m)\bload_extension\b',
        '(?m)\bATTACH\s+DATABASE\b',
        '(?m)journal_mode\s*=\s*WAL\b'
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
            $relativePath.StartsWith('apps/ipc-probe/', [System.StringComparison]::OrdinalIgnoreCase)
        if ($isRustProductSource) {
            $isIpcWinSys = $relativePath.Equals(
                'crates/ipc-win/src/sys.rs',
                [System.StringComparison]::OrdinalIgnoreCase
            )
            $isReviewedUnsafeBoundary =
                $relativePath.Equals(
                    'crates/clipboard-win/src/sys.rs',
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

    if ($violations.Count -gt 0) {
        $violations | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
        exit 1
    }

    Write-Host 'Source policy: PASS'
}
finally {
    Pop-Location
}
