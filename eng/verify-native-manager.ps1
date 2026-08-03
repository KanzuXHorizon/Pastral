[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Build', 'Smoke', 'All')]
    [string]$Mode = 'All'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$managerRoot = Join-Path $repositoryRoot 'apps\manager\Pastral.Manager'
$projectPath = Join-Path $managerRoot 'Pastral.Manager.vcxproj'

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
    Write-Host 'Pastral native manager static verification'

    $required = @(
        'Pastral.slnx',
        'Directory.Build.props',
        'Directory.Packages.props',
        'apps/manager/Pastral.Manager/Pastral.Manager.vcxproj',
        'apps/manager/Pastral.Manager/App.xaml',
        'apps/manager/Pastral.Manager/MainWindow.xaml',
        'apps/manager/Pastral.Manager/Themes/PastralTheme.xaml',
        'apps/manager/Pastral.Manager/Pages/HomePage.xaml',
        'apps/manager/Pastral.Manager/Pages/HistoryPage.xaml',
        'apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.idl',
        'apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.h',
        'apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.cpp',
        'apps/manager/Pastral.Manager/ViewModels/ManagerState.h',
        'apps/manager/Pastral.Manager/Services/IManagerDataProvider.h',
        'apps/manager/Pastral.Manager/Services/ManagerDataProvider.h',
        'apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp'
    )

    $missing = @(
        $required | Where-Object {
            -not (Test-Path -LiteralPath (Join-Path $repositoryRoot $_) -PathType Leaf)
        }
    )
    if ($missing.Count -gt 0) {
        Fail ('Missing native manager files: ' + ($missing -join ', '))
    }

    $textExtensions = @('.cpp', '.h', '.hpp', '.idl', '.xaml', '.vcxproj', '.props', '.resw', '.xml')
    $managerFiles = @(
        Get-ChildItem -LiteralPath $managerRoot -Recurse -File |
            Where-Object {
                ($textExtensions -contains $_.Extension.ToLowerInvariant()) -and
                ($_.FullName -notmatch '\\(?:obj|x64|bin|AppPackages|Generated Files)\\')
            }
    )

    $forbiddenPatterns = @(
        '(?i)\bWebView2?\b',
        '(?i)\bElectron\b',
        '(?i)\bTauri\b',
        '(?i)\brusqlite\b',
        '(?i)\bsqlite3?_open\b',
        '(?i)\bATTACH\s+DATABASE\b',
        '(?i)\bWinHttp(Open|Connect|SendRequest)\b',
        '(?i)\bInternet(Open|Connect)\b',
        '(?i)\bTODO\b',
        '(?i)\bTBD\b',
        '(?i)\bFIXME\b'
    )

    $violations = New-Object System.Collections.Generic.List[string]
    foreach ($file in $managerFiles) {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        foreach ($pattern in $forbiddenPatterns) {
            if ([System.Text.RegularExpressions.Regex]::IsMatch($content, $pattern)) {
                $relative = $file.FullName.Substring($repositoryRoot.Length + 1)
                $violations.Add("forbidden pattern '$pattern' in $relative")
            }
        }

        if ($file.Name -ne 'ManagerDataProvider.cpp' -and $content.Contains('synthetic-clip-')) {
            $relative = $file.FullName.Substring($repositoryRoot.Length + 1)
            $violations.Add("synthetic clip payload outside provider in $relative")
        }
    }

    if ($violations.Count -gt 0) {
        $violations | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
        exit 1
    }

    Assert-Contains $projectPath '<WindowsPackageType>None</WindowsPackageType>' 'WindowsPackageType=None'
    Assert-Contains $projectPath '<UseWinUI>true</UseWinUI>' 'UseWinUI=true'
    Assert-Contains $projectPath '<LanguageStandard>stdcpp20</LanguageStandard>' 'C++20 language standard'
    Assert-Contains $projectPath '<TreatWarningAsError>true</TreatWarningAsError>' 'warnings-as-errors'
    Assert-Contains $projectPath 'Debug\|x64' 'Debug x64 configuration'
    Assert-Contains $projectPath 'Release\|x64' 'Release x64 configuration'
    Assert-Contains $projectPath 'Microsoft\.WindowsAppSDK' 'Windows App SDK PackageReference'
    Assert-Contains $projectPath 'Microsoft\.Windows\.CppWinRT' 'C++/WinRT PackageReference'

    $packagesPath = Join-Path $repositoryRoot 'Directory.Packages.props'
    Assert-Contains $packagesPath 'Microsoft\.WindowsAppSDK"\s+Version="2\.3\.1"' 'Windows App SDK 2.3.1 pin'
    Assert-Contains $packagesPath 'Microsoft\.Windows\.CppWinRT"\s+Version="3\.0\.260715\.1"' 'C++/WinRT package pin'

    $mainWindow = Join-Path $managerRoot 'MainWindow.xaml'
    Assert-Contains $mainWindow '<MicaBackdrop' 'Mica backdrop'
    Assert-Contains $mainWindow '<TitleBar' 'TitleBar control'
    Assert-Contains $mainWindow '<NavigationView' 'NavigationView shell'
    Assert-Contains $mainWindow '<InfoBar' 'global InfoBar'
    Assert-Contains $mainWindow '<Frame' 'content Frame'
    Assert-Contains $mainWindow 'AutomationProperties\.Name=' 'shell accessibility names'

    $homePage = Join-Path $managerRoot 'Pages\HomePage.xaml'
    Assert-Contains $homePage 'HeadingLevel="Level1"' 'Home Level1 heading'
    Assert-Contains $homePage 'AutomationProperties\.Name=' 'Home accessibility names'
    Assert-Contains $homePage 'x:Name="HomeOperationalStateRegion"' 'Home operational state region'
    Assert-Contains $homePage 'x:Name="RetryConnectionButton"' 'Home recovery action'
    Assert-Contains $homePage 'x:Name="HomeRecentClipsList"' 'Home recent clips list'
    Assert-Contains $homePage 'x:Name="HomeEmptyStatePanel"' 'Home empty state panel'
    Assert-Contains $homePage 'x:Name="HomeSyntheticNotice"' 'Home synthetic-data disclosure'
    Assert-Contains $homePage 'Text="\{Binding SafePreview\}"' 'Home safe preview binding'
    Assert-Contains $homePage 'Text="\{Binding Source\}"' 'Home source binding'
    Assert-Contains $homePage 'Text="\{Binding RepresentationSummary\}"' 'Home representation binding'

    $homeCode = Join-Path $managerRoot 'Pages\HomePage.xaml.cpp'
    Assert-Contains $homeCode 'CreateManagerDataProvider\(' 'Home provider boundary'
    Assert-Contains $homeCode 'LoadSnapshot\(' 'Home snapshot loading'
    Assert-Contains $homeCode 'RetryConnection_Click' 'Home retry handler'

    $historyPage = Join-Path $managerRoot 'Pages\HistoryPage.xaml'
    Assert-Contains $historyPage 'HeadingLevel="Level1"' 'History Level1 heading'
    Assert-Contains $historyPage 'AutomationProperties\.Name=' 'History accessibility names'
    Assert-Contains $historyPage '<VisualStateManager\.VisualStateGroups>' 'History adaptive visual states'
    Assert-Contains $historyPage 'MinWindowWidth="920"' 'History wide-layout trigger'

    foreach ($page in @($homePage, $historyPage)) {
        $content = [System.IO.File]::ReadAllText($page)
        if ([System.Text.RegularExpressions.Regex]::IsMatch(
            $content,
            '(?i)(Background|Foreground)="#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})"'
        )) {
            Fail "Hard-coded page color found in $page"
        }
    }

    $viewModelIdl = Join-Path $managerRoot 'ViewModels\ClipPreviewViewModel.idl'
    foreach ($property in @(
        'String\s+Id\s*\{\s*get;',
        'String\s+SafePreview\s*\{\s*get;',
        'String\s+Source\s*\{\s*get;',
        'String\s+RelativeTime\s*\{\s*get;',
        'String\s+TypeLabel\s*\{\s*get;',
        'String\s+Profile\s*\{\s*get;',
        'String\s+RepresentationSummary\s*\{\s*get;',
        'String\s+AutomationName\s*\{\s*get;',
        'Boolean\s+Pinned\s*\{\s*get;',
        'Boolean\s+Unavailable\s*\{\s*get;'
    )) {
        Assert-Contains $viewModelIdl $property 'immutable clip preview property'
    }

    $provider = Join-Path $managerRoot 'Services\ManagerDataProvider.cpp'
    Assert-Contains $provider '#if\s+defined\(_DEBUG\)' 'Debug-only synthetic provider guard'
    Assert-Contains $provider '#else' 'Release provider branch'
    Assert-Contains $provider 'synthetic-clip-' 'bounded synthetic IDs'
    Assert-Contains $provider 'ConnectionState::Disconnected' 'Release disconnected state'
    Assert-Contains $provider 'snapshot\.synthetic\s*=\s*false' 'Release synthetic exclusion'

    Write-Host 'Native manager static policy: PASS'
}

function Resolve-MSBuild {
    $vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        Fail 'vswhere.exe was not found'
    }

    $installationPath = (& $vswhere -latest -products Microsoft.VisualStudio.Product.BuildTools -property installationPath).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($installationPath)) {
        Fail 'Visual Studio Build Tools installation was not found'
    }

    $msbuild = Join-Path $installationPath 'MSBuild\Current\Bin\MSBuild.exe'
    if (-not (Test-Path -LiteralPath $msbuild -PathType Leaf)) {
        Fail "MSBuild.exe was not found at $msbuild"
    }
    return $msbuild
}

function Invoke-BuildVerification {
    if (-not (Test-Path -LiteralPath $projectPath -PathType Leaf)) {
        Fail "Manager project was not found at $projectPath"
    }

    $msbuild = Resolve-MSBuild
    $lockFile = Join-Path $managerRoot 'packages.lock.json'
    $locked = if (Test-Path -LiteralPath $lockFile -PathType Leaf) { 'true' } else { 'false' }

    foreach ($configuration in @('Debug', 'Release')) {
        Write-Host "Building manager $configuration|x64"
        & $msbuild $projectPath '/restore' '/m:1' '/nr:false' '/nologo' '/verbosity:minimal' `
            "/p:Configuration=$configuration" '/p:Platform=x64' `
            "/p:RestoreLockedMode=$locked"
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }

    Write-Host 'Native manager Debug and Release builds: PASS'
}

function Resolve-DebugExecutable {
    $candidates = @(
        (Join-Path $managerRoot 'x64\Debug\pastral-manager.exe'),
        (Join-Path $managerRoot 'bin\x64\Debug\pastral-manager.exe')
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }
    Fail ('Debug manager executable not found. Checked: ' + ($candidates -join ', '))
}

function Invoke-SmokeVerification {
    $runtime = @(Get-AppxPackage -Name 'Microsoft.WindowsAppRuntime.2' -ErrorAction SilentlyContinue |
        Where-Object { $_.Architecture -eq 'X64' -and $_.Version -eq [version]'2.3.1.0' })
    if ($runtime.Count -eq 0) {
        Fail 'Microsoft.WindowsAppRuntime.2 x64 version 2.3.1.0 is required for manager smoke testing'
    }

    $executable = Resolve-DebugExecutable
    $process = Start-Process -FilePath $executable -PassThru
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        $windowHandle = [IntPtr]::Zero
        while ([DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
            Start-Sleep -Milliseconds 200
            $process.Refresh()
            $windowHandle = $process.MainWindowHandle
            if ($windowHandle -ne [IntPtr]::Zero) {
                break
            }
        }

        if ($process.HasExited) {
            Fail "Manager exited during smoke test with code $($process.ExitCode)"
        }
        if ($windowHandle -eq [IntPtr]::Zero) {
            Fail 'Manager did not create a top-level window within 15 seconds'
        }

        Start-Sleep -Seconds 2
        $process.Refresh()
        if ($process.HasExited) {
            Fail "Manager exited before responsiveness interval completed with code $($process.ExitCode)"
        }

        Write-Host "Manager smoke window handle: $windowHandle"
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(5000)) {
            $process.Kill()
            $process.WaitForExit()
            Fail 'Manager did not close within five seconds'
        }
    }
    finally {
        if (-not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
        $process.Dispose()
    }

    Write-Host 'Native manager runtime smoke: PASS'
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
