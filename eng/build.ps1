[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Verify', 'Format', 'Check', 'Test', 'Storage', 'Clipboard', 'IpcPrototype', 'IpcTransport', 'Agent', 'AgentPolicy', 'Manager', 'ManagerBuild', 'NativePolicy', 'Clippy', 'Doc', 'Dependencies', 'SourcePolicy', 'All', 'Full')]
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
    $global:LASTEXITCODE = 0
    & $Action
    $stepSucceeded = $?
    $exitCode = $global:LASTEXITCODE
    if (-not $stepSucceeded) {
        exit $(if ($exitCode -ne 0) { $exitCode } else { 1 })
    }
    if ($exitCode -ne 0) {
        exit $exitCode
    }
}

function Invoke-Verify { Invoke-Step 'Verify Rust toolchain' { & "$PSScriptRoot\verify-toolchain.ps1" } }
function Invoke-VerifyNative { Invoke-Step 'Verify Rust and native manager toolchains' { & "$PSScriptRoot\verify-toolchain.ps1" -RequireNativeManager } }
function Invoke-Format { Invoke-Step 'Check formatting' { cargo fmt --all -- --check } }
function Invoke-Check { Invoke-Step 'Check workspace' { cargo check --locked --workspace --all-targets } }
function Invoke-Test { Invoke-Step 'Test workspace' { cargo test --locked --workspace --all-targets } }
function Invoke-Storage { Invoke-Step 'Test storage foundation' { cargo test --locked -p pastral-storage --all-targets } }
function Invoke-Clipboard { Invoke-Step 'Test Win32 clipboard foundation' { cargo test --locked -p pastral-clipboard-win --all-targets } }
function Invoke-IpcPrototype { Invoke-Step 'Verify IPC framing and schema prototype' { & "$PSScriptRoot\verify-ipc-prototype.ps1" -Mode All } }
function Invoke-IpcTransport { Invoke-Step 'Verify authenticated IPC transport' { & "$PSScriptRoot\verify-ipc-transport.ps1" -Mode All } }
function Invoke-Agent { Invoke-Step 'Verify diagnostic resident agent' { & "$PSScriptRoot\verify-agent.ps1" -Mode All } }
function Invoke-AgentPolicy { Invoke-Step 'Verify diagnostic resident agent policy' { & "$PSScriptRoot\verify-agent.ps1" -Mode Static } }
function Invoke-Manager { Invoke-Step 'Verify native manager including runtime smoke' { & "$PSScriptRoot\verify-native-manager.ps1" -Mode All } }
function Invoke-ManagerBuild { Invoke-Step 'Build native manager Debug and Release' { & "$PSScriptRoot\verify-native-manager.ps1" -Mode Build } }
function Invoke-NativePolicy { Invoke-Step 'Verify native manager policy' { & "$PSScriptRoot\verify-native-manager.ps1" -Mode Static } }
function Invoke-Clippy { Invoke-Step 'Clippy workspace' { cargo clippy --locked --workspace --all-targets --all-features -- -D warnings } }
function Invoke-Doc { Invoke-Step 'Build documentation' { cargo doc --locked --workspace --no-deps } }
function Invoke-Dependencies { Invoke-Step 'Verify dependency policy' { & "$PSScriptRoot\verify-dependencies.ps1" } }
function Invoke-SourcePolicy { Invoke-Step 'Verify source policy' { & "$PSScriptRoot\verify-source-policy.ps1" } }

switch ($Task) {
    'Verify' { Invoke-Verify }
    'Format' { Invoke-Format }
    'Check' { Invoke-Check }
    'Test' { Invoke-Test }
    'Storage' { Invoke-Storage }
    'Clipboard' { Invoke-Clipboard }
    'IpcPrototype' { Invoke-IpcPrototype }
    'IpcTransport' { Invoke-IpcTransport }
    'Agent' { Invoke-Agent }
    'AgentPolicy' { Invoke-AgentPolicy }
    'Manager' { Invoke-Manager }
    'ManagerBuild' { Invoke-ManagerBuild }
    'NativePolicy' { Invoke-NativePolicy }
    'Clippy' { Invoke-Clippy }
    'Doc' { Invoke-Doc }
    'Dependencies' { Invoke-Dependencies }
    'SourcePolicy' { Invoke-SourcePolicy }
    'All' {
        Invoke-Verify
        Invoke-Format
        Invoke-Check
        Invoke-Test
        Invoke-Clippy
        Invoke-Doc
        Invoke-Dependencies
        Invoke-SourcePolicy
    }
    'Full' {
        Invoke-VerifyNative
        Invoke-Format
        Invoke-Check
        Invoke-Test
        Invoke-Clippy
        Invoke-Doc
        Invoke-Dependencies
        Invoke-SourcePolicy
        Invoke-IpcPrototype
        Invoke-IpcTransport
        Invoke-Agent
        Invoke-NativePolicy
        Invoke-ManagerBuild
    }
}
