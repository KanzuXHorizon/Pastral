[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Verify', 'Format', 'Check', 'Test', 'Clippy', 'Doc', 'All')]
    [string]$Task = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    Write-Host "==> $Name"
    & $Action
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

function Invoke-Verify { Invoke-Step 'Verify toolchain' { & "$PSScriptRoot\verify-toolchain.ps1" } }
function Invoke-Format { Invoke-Step 'Check formatting' { cargo fmt --all -- --check } }
function Invoke-Check { Invoke-Step 'Check workspace' { cargo check --workspace --all-targets } }
function Invoke-Test { Invoke-Step 'Test workspace' { cargo test --workspace --all-targets } }
function Invoke-Clippy { Invoke-Step 'Clippy workspace' { cargo clippy --workspace --all-targets --all-features -- -D warnings } }
function Invoke-Doc { Invoke-Step 'Build documentation' { cargo doc --workspace --no-deps } }

switch ($Task) {
    'Verify' { Invoke-Verify }
    'Format' { Invoke-Format }
    'Check' { Invoke-Check }
    'Test' { Invoke-Test }
    'Clippy' { Invoke-Clippy }
    'Doc' { Invoke-Doc }
    'All' {
        Invoke-Verify
        Invoke-Format
        Invoke-Check
        Invoke-Test
        Invoke-Clippy
        Invoke-Doc
    }
}
