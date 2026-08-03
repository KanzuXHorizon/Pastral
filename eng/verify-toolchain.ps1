[CmdletBinding()]
param(
    [Parameter()][switch]$RequireNativeManager
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Get-CommandOutput {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter()][string[]]$Arguments = @()
    )

    $output = & $FilePath @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "Command failed with exit code ${exitCode}: $FilePath $($Arguments -join ' ')`n$($output -join "`n")"
    }
    return ($output -join "`n").Trim()
}

function Add-Failure {
    param(
        [Parameter(Mandatory = $true)][System.Collections.Generic.List[string]]$List,
        [Parameter(Mandatory = $true)][string]$Message
    )

    $List.Add($Message)
}

$requiredChannel = '1.97.1-x86_64-pc-windows-msvc'
$requiredTarget = 'x86_64-pc-windows-msvc'
$currentNativeSdk = '10.0.26100.0'
$deferredReleaseSdk = '10.0.28000.2526'
$requiredRuntimeName = 'Microsoft.WindowsAppRuntime.2'
$requiredRuntimeVersion = [version]'2.3.1.0'

Write-Host 'Pastral toolchain verification'
Write-Host 'Classification: RequiredForRustFoundation'

$rustFailures = New-Object System.Collections.Generic.List[string]
$rustcVersion = Get-CommandOutput 'rustc' @('-Vv')
$cargoVersion = Get-CommandOutput 'cargo' @('-V')
$rustfmtVersion = Get-CommandOutput 'rustfmt' @('--version')
$clippyVersion = Get-CommandOutput 'cargo' @('clippy', '--version')
$activeToolchain = Get-CommandOutput 'rustup' @('show', 'active-toolchain')
$hostLine = ($rustcVersion -split "`r?`n" | Where-Object { $_ -like 'host:*' } | Select-Object -First 1)
$actualTarget = if ($hostLine) { ($hostLine -replace '^host:\s*', '').Trim() } else { '' }

Write-Host "rustc:`n$rustcVersion"
Write-Host "cargo: $cargoVersion"
Write-Host "rustup active toolchain: $activeToolchain"
Write-Host "target: $actualTarget"
Write-Host "rustfmt: $rustfmtVersion"
Write-Host "clippy: $clippyVersion"

if ($rustcVersion -notmatch '^rustc 1\.97\.1 ') {
    Add-Failure $rustFailures 'rustc must be 1.97.1'
}
if ($activeToolchain -notmatch '^1\.97\.1-x86_64-pc-windows-msvc\s') {
    Add-Failure $rustFailures "active toolchain must be $requiredChannel"
}
if ($actualTarget -ne $requiredTarget) {
    Add-Failure $rustFailures "host/target must be $requiredTarget"
}
if ($clippyVersion -notmatch '^clippy 0\.1\.97 ') {
    Add-Failure $rustFailures 'Clippy must be the Rust 1.97.1 component'
}

Write-Host 'Classification: NativeManagerBuild'
$nativeFailures = New-Object System.Collections.Generic.List[string]
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsInstall = $null
$vsVersion = $null
$msbuild = $null

if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    Write-Host 'Visual Studio locator: not found'
    if ($RequireNativeManager) {
        Add-Failure $nativeFailures 'vswhere.exe was not found'
    }
} else {
    $vsInstall = (& $vswhere -latest -products * -requires `
        Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
        -property installationPath 2>$null | Select-Object -First 1)
    $vsVersion = (& $vswhere -latest -products * -requires `
        Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
        -property installationVersion 2>$null | Select-Object -First 1)

    if ([string]::IsNullOrWhiteSpace($vsInstall)) {
        Write-Host 'Visual Studio C++ WinUI build tools: not detected'
        if ($RequireNativeManager) {
            Add-Failure $nativeFailures 'Visual Studio 2022 with MSVC x64 and C++ WinUI build tools was not found'
        }
    } else {
        Write-Host "Visual Studio: $vsVersion"
        Write-Host "Visual Studio path: $vsInstall"

        $msbuild = Join-Path $vsInstall 'MSBuild\Current\Bin\MSBuild.exe'
        if (Test-Path -LiteralPath $msbuild -PathType Leaf) {
            $msbuildVersion = Get-CommandOutput $msbuild @('-version', '-nologo')
            Write-Host "MSBuild: $msbuildVersion"
        } else {
            Write-Host "MSBuild: missing at $msbuild"
            if ($RequireNativeManager) {
                Add-Failure $nativeFailures 'MSBuild.exe was not found in the selected Visual Studio installation'
            }
        }

        $toolsetRoot = Join-Path $vsInstall 'VC\Tools\MSVC'
        $compiler = Get-ChildItem -LiteralPath $toolsetRoot -Filter cl.exe -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\Hostx64\\x64\\cl\.exe$' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        $linker = Get-ChildItem -LiteralPath $toolsetRoot -Filter link.exe -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\Hostx64\\x64\\link\.exe$' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        $uwpToolset = Join-Path $vsInstall 'MSBuild\Microsoft\VC\v170\Application Type\Windows Store\10.0\Platforms\x64\PlatformToolsets\v143\Toolset.props'

        if ($null -eq $compiler) {
            Write-Host 'MSVC x64 compiler: not found'
            if ($RequireNativeManager) {
                Add-Failure $nativeFailures 'MSVC x64 compiler cl.exe was not found'
            }
        } else {
            Write-Host "MSVC compiler: $($compiler.FullName)"
        }
        if ($null -eq $linker) {
            Write-Host 'MSVC x64 linker: not found'
            if ($RequireNativeManager) {
                Add-Failure $nativeFailures 'MSVC x64 linker link.exe was not found'
            }
        } else {
            Write-Host "MSVC linker: $($linker.FullName)"
        }
        if (Test-Path -LiteralPath $uwpToolset -PathType Leaf) {
            Write-Host "C++ WinUI/UWP x64 v143 toolset: $uwpToolset"
        } else {
            Write-Host 'C++ WinUI/UWP x64 v143 toolset: not found'
            if ($RequireNativeManager) {
                Add-Failure $nativeFailures 'C++ WinUI/UWP x64 v143 platform toolset was not found'
            }
        }
    }
}

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
$sdkVersions = @()
if (Test-Path -LiteralPath $sdkRoot -PathType Container) {
    $sdkVersions = @(Get-ChildItem -LiteralPath $sdkRoot -Directory | Select-Object -ExpandProperty Name | Sort-Object)
}
Write-Host "Windows SDKs: $(if ($sdkVersions.Count -gt 0) { $sdkVersions -join ', ' } else { 'none detected' })"
if ($sdkVersions -contains $currentNativeSdk) {
    Write-Host "Current native manager SDK ${currentNativeSdk}: present"
} else {
    Write-Host "Current native manager SDK ${currentNativeSdk}: not present"
    if ($RequireNativeManager) {
        Add-Failure $nativeFailures "Windows SDK $currentNativeSdk is required for the native manager build"
    }
}

Write-Host 'Classification: NativeManagerSmokeRuntime'
$runtimePackages = @(Get-AppxPackage -Name $requiredRuntimeName -ErrorAction SilentlyContinue |
    Where-Object { $_.Architecture -eq 'X64' } |
    Sort-Object Version -Descending)
if ($runtimePackages.Count -eq 0) {
    Write-Host "$requiredRuntimeName x64: not installed; compile gates can still run, local Smoke cannot"
} else {
    Write-Host "$requiredRuntimeName x64 versions: $((@($runtimePackages | ForEach-Object { $_.Version.ToString() }) | Sort-Object -Unique) -join ', ')"
    if (@($runtimePackages | Where-Object { $_.Version -eq $requiredRuntimeVersion }).Count -gt 0) {
        Write-Host "Native manager smoke runtime ${requiredRuntimeVersion}: present"
    } else {
        Write-Host "Native manager smoke runtime ${requiredRuntimeVersion}: not present; local Smoke will fail"
    }
}

Write-Host 'Classification: DeferredForPackagingAndRelease'
if ($sdkVersions -contains $deferredReleaseSdk) {
    Write-Host "Deferred release SDK ${deferredReleaseSdk}: present"
} else {
    Write-Host "Deferred release SDK ${deferredReleaseSdk}: not present; accepted for the current unpackaged manager slice"
}

if ($rustFailures.Count -gt 0) {
    $rustFailures | ForEach-Object { Write-Error "RequiredForRustFoundation mismatch: $_" }
    exit 1
}
if ($RequireNativeManager -and $nativeFailures.Count -gt 0) {
    $nativeFailures | ForEach-Object { Write-Error "NativeManagerBuild mismatch: $_" }
    exit 1
}

Write-Host 'RequiredForRustFoundation: PASS'
if ($RequireNativeManager) {
    Write-Host 'NativeManagerBuild: PASS'
} else {
    Write-Host 'NativeManagerBuild: reported only; pass -RequireNativeManager to enforce it'
}
exit 0
