[CmdletBinding()]
param(
    [Parameter()][ValidateSet('Static', 'Build', 'Smoke', 'All')]
    [string]$Mode = 'All',
    [Parameter()][string]$SmokeExecutable = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$managerRoot = Join-Path $repositoryRoot 'apps\manager\Pastral.Manager'
$projectPath = Join-Path $managerRoot 'Pastral.Manager.vcxproj'
$verificationRoot = Join-Path $repositoryRoot ('target\verification\pastral-native-manager-' + [guid]::NewGuid().ToString('N'))
$debugOutput = Join-Path $verificationRoot 'debug-out'
$debugIntermediate = Join-Path $verificationRoot 'debug-obj'
$releaseOutput = Join-Path $verificationRoot 'release-out'
$releaseIntermediate = Join-Path $verificationRoot 'release-obj'
$nativeBuildMutexName = 'Local\Pastral.NativeManager.Build'

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

function Invoke-WithNativeBuildLock {
    param([Parameter(Mandatory = $true)][scriptblock]$Action)

    $mutex = [System.Threading.Mutex]::new($false, $nativeBuildMutexName)
    $acquired = $false
    try {
        try {
            $acquired = $mutex.WaitOne([TimeSpan]::FromMinutes(10))
        }
        catch [System.Threading.AbandonedMutexException] {
            $acquired = $true
        }
        if (-not $acquired) {
            Fail "Timed out waiting for native manager build lock: $nativeBuildMutexName"
        }
        & $Action
    }
    finally {
        if ($acquired) {
            [void]$mutex.ReleaseMutex()
        }
        $mutex.Dispose()
    }
}

function Assert-NotContains {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$Description
    )

    $content = [System.IO.File]::ReadAllText($Path)
    if ([System.Text.RegularExpressions.Regex]::IsMatch(
        $content,
        $Pattern,
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )) {
        Fail "$Description was found in $Path"
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
        'apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp',
        'apps/manager/Pastral.Manager/Services/ManagerIpcBridge.h',
        'apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp',
        'apps/manager/Pastral.Manager/Tests/ManagerIpcBridgeProbe.cpp',
        'apps/manager/Pastral.Manager/Tests/Pastral.Manager.IpcProbe.vcxproj'
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
    Assert-Contains $projectPath 'Services\\ManagerIpcBridge\.cpp' 'manager IPC bridge source'
    Assert-Contains $projectPath 'manager-ipc-bridge\\include' 'manager IPC bridge header path'
    Assert-Contains $PSCommandPath 'Local\\Pastral\.NativeManager\.Build' 'shared native XAML build mutex'
    Assert-Contains $PSCommandPath 'target\\verification\\pastral-native-manager-' 'per-run native verification root'
    Assert-Contains $PSCommandPath '\[string\]\$SmokeExecutable' 'explicit isolated smoke executable override'

    $bridgeCode = Join-Path $managerRoot 'Services\ManagerIpcBridge.cpp'
    Assert-Contains $bridgeCode 'GetModuleFileNameW' 'executable-directory bridge resolution'
    Assert-Contains $bridgeCode 'pastral-manager-ipc-bridge\.dll' 'exact bridge filename'
    Assert-Contains $bridgeCode 'LoadLibraryExW' 'explicit bridge load API'
    Assert-Contains $bridgeCode 'LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR\s*\|\s*LOAD_LIBRARY_SEARCH_SYSTEM32' 'restricted DLL dependency search'
    foreach ($symbol in @(
        'pastral_manager_ipc_abi_version',
        'pastral_manager_ipc_result_size',
        'pastral_manager_ipc_health_w'
    )) {
        Assert-Contains $bridgeCode $symbol 'exact bridge symbol resolution'
    }

    $packagesPath = Join-Path $repositoryRoot 'Directory.Packages.props'
    Assert-Contains $packagesPath 'Microsoft\.WindowsAppSDK"\s+Version="2\.3\.1"' 'Windows App SDK 2.3.1 pin'
    Assert-Contains $packagesPath 'Microsoft\.Windows\.CppWinRT"\s+Version="3\.0\.260715\.1"' 'C++/WinRT package pin'

    $mainWindow = Join-Path $managerRoot 'MainWindow.xaml'
    Assert-Contains $mainWindow '<MicaBackdrop' 'Mica backdrop'
    Assert-Contains $mainWindow '<TitleBar' 'TitleBar control'
    Assert-Contains $mainWindow 'x:Name="AppTitleBar"[\s\S]*x:Uid="AppTitleBar"[\s\S]*Title="Pastral"' 'single localized title-bar brand'
    Assert-NotContains $mainWindow '<NavigationView\.PaneHeader>' 'duplicate sidebar branding'
    Assert-NotContains $mainWindow 'x:Uid="ShellNavigationView"' 'localized duplicate navigation pane title'
    Assert-Contains $mainWindow 'OpenPaneLength="248"' 'balanced expanded navigation width'
    Assert-Contains $mainWindow 'CompactPaneLength="48"' 'compact navigation width'
    Assert-Contains $mainWindow '<NavigationView' 'NavigationView shell'
    Assert-Contains $mainWindow '<InfoBar' 'global InfoBar'
    Assert-Contains $mainWindow '<Frame' 'content Frame'
    Assert-Contains $mainWindow 'AutomationProperties\.Name=' 'shell accessibility names'
    Assert-Contains $mainWindow 'x:Name="GlobalStatusBar"[\s\S]*Visibility="Collapsed"' 'non-duplicative global status default'
    Assert-NotContains $mainWindow 'Live clipboard history remains disconnected until local IPC is implemented' 'obsolete disconnected shell copy'

    $englishResources = Join-Path $managerRoot 'Strings\en-US\Resources.resw'
    $vietnameseResources = Join-Path $managerRoot 'Strings\vi-VN\Resources.resw'
    Assert-Contains $englishResources 'HomePageEyebrow\.Text"[\s\S]*<value>Private by design</value>' 'English Home positioning copy'
    Assert-Contains $vietnameseResources 'HomePageEyebrow\.Text"[\s\S]*<value>Riêng tư từ thiết kế</value>' 'Vietnamese Home positioning copy'
    Assert-Contains $englishResources 'HistoryPageEyebrow\.Text"[\s\S]*<value>Search with context</value>' 'English History positioning copy'
    Assert-Contains $vietnameseResources 'HistoryPageEyebrow\.Text"[\s\S]*<value>Tìm kiếm theo ngữ cảnh</value>' 'Vietnamese History positioning copy'

    $homePage = Join-Path $managerRoot 'Pages\HomePage.xaml'
    Assert-Contains $homePage 'HeadingLevel="Level1"' 'Home Level1 heading'
    Assert-Contains $homePage 'AutomationProperties\.Name=' 'Home accessibility names'
    Assert-Contains $homePage 'x:Name="HomeLayoutStates"' 'Home adaptive visual states'
    Assert-Contains $homePage 'MinWindowWidth="720"' 'Home wide-layout trigger'
    Assert-Contains $homePage 'x:Name="HomeHeroIcon"' 'Home responsive hero mark'
    Assert-Contains $homePage 'x:Name="HomeRecentHeader"' 'Home responsive recent header'
    Assert-Contains $homePage 'x:Name="HomeOperationalStateRegion"' 'Home operational state region'
    Assert-Contains $homePage 'x:Name="HomeLoadingIndicator"' 'Home loading indicator'
    Assert-Contains $homePage 'x:Name="HomeOverviewRegion"' 'Home consolidated overview region'
    Assert-Contains $homePage 'x:Name="HomeEmptyStateTitle"' 'Home contextual empty-state title'
    Assert-Contains $homePage 'x:Name="HomeEmptyStateDetail"' 'Home contextual empty-state detail'
    Assert-NotContains $homePage '<ItemsWrapGrid' 'fixed-width Home card grid'
    Assert-Contains $homePage 'x:Name="RetryConnectionButton"' 'Home recovery action'
    Assert-Contains $homePage 'x:Name="HomeRecentClipsList"' 'Home recent clips list'
    Assert-Contains $homePage 'x:Name="HomeEmptyStatePanel"' 'Home empty state panel'
    Assert-Contains $homePage 'x:Name="HomeSyntheticNotice"' 'Home synthetic-data disclosure'
    Assert-Contains $homePage 'Text="\{Binding SafePreview\}"' 'Home safe preview binding'
    Assert-Contains $homePage 'Text="\{Binding Source\}"' 'Home source binding'
    Assert-Contains $homePage 'Text="\{Binding RepresentationSummary\}"' 'Home representation binding'

    $provider = Join-Path $managerRoot 'Services\ManagerDataProvider.cpp'
    Assert-Contains $provider 'CreateLoadingSnapshot\(\)[\s\S]*statusDetail\s*=\s*L"Checking the secure local connection\."' 'plain-language loading connection copy'
    Assert-Contains $provider 'CreateLoadingSnapshot\(\)[\s\S]*storageSummary\s*=\s*L"Preparing local status"' 'plain-language loading storage copy'

    $providerInterface = Join-Path $managerRoot 'Services\IManagerDataProvider.h'
    Assert-Contains $providerInterface 'LoadSnapshotAsync\(SnapshotCompletion completion\)' 'asynchronous manager provider contract'
    Assert-Contains $providerInterface 'RefreshAsync\(SnapshotCompletion completion\)' 'asynchronous manager refresh contract'
    Assert-Contains $providerInterface 'SearchAsync\(std::wstring query, SnapshotCompletion completion\)' 'asynchronous manager Search contract'

    $homeCode = Join-Path $managerRoot 'Pages\HomePage.xaml.cpp'
    Assert-Contains $homeCode 'CreateManagerDataProvider\(' 'Home provider boundary'
    Assert-Contains $homeCode 'CreateLoadingSnapshot\(' 'Home immediate Loading state'
    Assert-Contains $homeCode 'LoadSnapshotAsync\(' 'Home asynchronous snapshot loading'
    Assert-Contains $homeCode 'get_weak\(' 'Home weak page reference'
    Assert-Contains $homeCode 'DispatcherQueue\(\)' 'Home UI-thread dispatcher capture'
    Assert-Contains $homeCode 'm_loadGeneration\s*==\s*generation' 'Home stale-result rejection'
    Assert-Contains $homeCode 'RetryConnection_Click' 'Home retry handler'
    Assert-Contains $homeCode 'HomeLoadingIndicator\(\)\.IsActive' 'Home loading progress state'
    Assert-Contains $homeCode 'snapshot\.connection\s*!=\s*ConnectionState::Loading[\s\S]*snapshot\.connection\s*!=\s*ConnectionState::Connected' 'non-duplicative Home connection banner policy'
    Assert-Contains $homeCode 'HomeEmptyStateTitle\(\)\.Text' 'Home contextual empty-state copy'

    $historyPage = Join-Path $managerRoot 'Pages\HistoryPage.xaml'
    Assert-Contains $historyPage 'HeadingLevel="Level1"' 'History Level1 heading'
    Assert-Contains $historyPage 'AutomationProperties\.Name=' 'History accessibility names'
    Assert-Contains $historyPage '<VisualStateManager\.VisualStateGroups>' 'History adaptive visual states'
    Assert-Contains $historyPage 'MinWindowWidth="0"' 'History narrow-layout trigger'
    Assert-Contains $historyPage 'MinWindowWidth="920"' 'History wide-layout trigger'
    Assert-NotContains $historyPage '<ScrollViewer\s+AutomationProperties\.Name="History page content"' 'page-level History scroll owner'
    Assert-NotContains $historyPage 'MaxHeight="560"' 'fixed-height History list'
    Assert-Contains $historyPage 'x:Name="HistoryBackButton"' 'History narrow detail Back action'
    Assert-Contains $historyPage 'x:Name="HistoryHeaderStates"' 'History responsive header states'
    Assert-Contains $historyPage 'x:Name="HistoryHeader"' 'History responsive header region'
    Assert-NotContains $historyPage 'SEARCH • RECALL • PASTE' 'premature History paste promise'
    Assert-Contains $historyPage 'x:Name="HistorySearchBox"' 'History search box'
    Assert-Contains $historyPage 'x:Name="HistoryResultsList"' 'History results list'
    Assert-Contains $historyPage 'x:Name="HistoryResultCount"' 'History result-count live region'
    Assert-Contains $historyPage 'x:Name="HistoryDetailsRegion"' 'History details region'
    Assert-Contains $historyPage 'x:Name="HistoryNoResultsPanel"' 'History no-results panel'
    Assert-Contains $historyPage 'x:Name="HistoryLoadingIndicator"' 'History loading indicator'
    Assert-Contains $historyPage 'x:Name="HistoryCommandBar"' 'History responsive command region'
    Assert-Contains $historyPage 'MinWindowWidth="640"' 'History command-layout trigger'
    Assert-Contains $historyPage 'x:Name="HistorySyntheticNotice"' 'History synthetic-data disclosure'
    Assert-Contains $historyPage 'x:Name="HistoryPasteButton"' 'History paste action'
    Assert-Contains $historyPage 'x:Name="HistoryCopyButton"' 'History copy action'
    Assert-Contains $historyPage 'AutomationProperties\.HelpText=' 'History disabled-action explanations'
    Assert-Contains $historyPage 'Text="\{Binding SafePreview\}"' 'History safe preview binding'
    Assert-Contains $historyPage 'Text="\{Binding Source\}"' 'History source binding'
    Assert-Contains $historyPage 'Text="\{Binding RepresentationSummary\}"' 'History representation binding'

    $historyCode = Join-Path $managerRoot 'Pages\HistoryPage.xaml.cpp'
    Assert-Contains $historyCode 'CreateManagerDataProvider\(' 'History provider boundary'
    Assert-Contains $historyCode 'CreateLoadingSnapshot\(' 'History immediate Loading state'
    Assert-Contains $historyCode 'LoadSnapshotAsync\(' 'History asynchronous snapshot loading'
    Assert-Contains $historyCode 'get_weak\(' 'History weak page reference'
    Assert-Contains $historyCode 'DispatcherQueue\(\)' 'History UI-thread dispatcher capture'
    Assert-Contains $historyCode 'm_loadGeneration\s*==\s*generation' 'History stale-result rejection'
    Assert-Contains $historyCode 'SearchBox_TextChanged' 'History search handler'
    Assert-Contains $historyCode 'CreateTimer\(' 'History debounce timer'
    Assert-Contains $historyCode 'std::chrono::milliseconds\(250\)' 'History 250 millisecond debounce'
    Assert-Contains $historyCode 'SearchAsync\(' 'History backend Search dispatch'
    Assert-Contains $historyCode 'RefreshAsync\(' 'History backend refresh dispatch'
    Assert-Contains $historyCode 'ResultsList_SelectionChanged' 'History selection handler'
    Assert-Contains $historyCode 'BackToResults_Click' 'History narrow Back handler'
    Assert-Contains $historyCode 'Page_SizeChanged' 'History responsive size handler'
    Assert-Contains $historyCode 'UpdateResponsiveLayout' 'History responsive render method'
    Assert-Contains $historyCode 'ClearFilters_Click' 'History clear-filter handler'
    Assert-Contains $historyCode 'Retry_Click' 'History retry handler'
    Assert-Contains $historyCode 'HistoryLoadingIndicator\(\)\.IsActive' 'History loading progress state'
    Assert-Contains $historyCode 'm_connection\s*!=\s*ConnectionState::Loading[\s\S]*m_connection\s*!=\s*ConnectionState::Connected' 'non-duplicative History connection banner policy'

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
    Assert-Contains $provider 'if\s*\(!diagnosticFlag\.has_value\(\)\)[\s\S]*return SyntheticSnapshot\(' 'Debug synthetic mode independent of local data-root resolution'
    Assert-Contains $provider 'PASTRAL_MANAGER_DIAGNOSTIC' 'diagnostic live-mode gate'
    Assert-Contains $provider 'std::thread\s+m_worker' 'single persistent provider worker'
    Assert-Contains $provider 'm_pending\s*=\s*PendingRequest' 'latest pending request replacement'
    Assert-Contains $provider 'request\.generation\s*==\s*m_generation' 'provider stale-result rejection'
    Assert-Contains $provider 'ManagerIpcBridge::QueryHistory\(' 'provider History IPC boundary'
    Assert-Contains $provider 'ManagerIpcBridge::QuerySearch\(' 'provider Search IPC boundary'
    Assert-Contains $provider 'hasMore' 'provider partial-page state'
    Assert-NotContains $provider 'Storage::open|sqlite3_|rusqlite' 'direct manager storage access'
    Assert-Contains $provider 'synthetic-clip-' 'bounded synthetic IDs'
    Assert-Contains $provider 'ConnectionState::Disconnected' 'live disconnected state'
    Assert-Contains $provider 'snapshot\.synthetic\s*=\s*false' 'live synthetic exclusion'

    Assert-Contains $PSCommandPath '"/p:OutDir=\$output"' 'isolated native manager output override'
    Assert-Contains $PSCommandPath '"/p:IntDir=\$intermediate"' 'isolated native manager intermediate override'
    Assert-Contains $PSCommandPath 'Join-Path \$debugOutput ''pastral-manager\.exe''' 'isolated Debug smoke executable'

    Write-Host 'Native manager static policy: PASS'
}

function Resolve-MSBuild {
    $vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        Fail 'vswhere.exe was not found'
    }

    $installationPath = (& $vswhere -latest -products * -requires `
        Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
        -property installationPath).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($installationPath)) {
        Fail 'Visual Studio 2022 with MSVC x64 and C++ WinUI build tools was not found'
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

    Invoke-WithNativeBuildLock {
        foreach ($configuration in @('Debug', 'Release')) {
            $outputDirectory = if ($configuration -eq 'Debug') { $debugOutput } else { $releaseOutput }
            $intermediateDirectory = if ($configuration -eq 'Debug') { $debugIntermediate } else { $releaseIntermediate }
            New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
            New-Item -ItemType Directory -Path $intermediateDirectory -Force | Out-Null
            $output = $outputDirectory.TrimEnd('\') + '\'
            $intermediate = $intermediateDirectory.TrimEnd('\') + '\'

            Write-Host "Building manager $configuration|x64"
            & $msbuild $projectPath '/restore' '/m:1' '/nr:false' '/nologo' '/verbosity:quiet' `
                "/p:Configuration=$configuration" '/p:Platform=x64' `
                "/p:RestoreLockedMode=$locked" "/p:OutDir=$output" "/p:IntDir=$intermediate"
            if ($LASTEXITCODE -ne 0) {
                throw "Manager $configuration build failed with exit code $LASTEXITCODE"
            }

            $executable = Join-Path $outputDirectory 'pastral-manager.exe'
            if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
                Fail "Manager $configuration executable was not produced at $executable"
            }
        }
    }

    Write-Host 'Native manager Debug and Release builds: PASS'
}

function Resolve-DebugExecutable {
    if (-not [string]::IsNullOrWhiteSpace($SmokeExecutable)) {
        if (Test-Path -LiteralPath $SmokeExecutable -PathType Leaf) {
            return $SmokeExecutable
        }
        Fail "Requested smoke executable was not found at $SmokeExecutable"
    }

    $freshExecutable = Join-Path $debugOutput 'pastral-manager.exe'
    if (-not (Test-Path -LiteralPath $freshExecutable -PathType Leaf)) {
        Invoke-BuildVerification
    }
    if (-not (Test-Path -LiteralPath $freshExecutable -PathType Leaf)) {
        Fail "Fresh Debug manager executable was not produced at $freshExecutable"
    }
    return $freshExecutable
}

function Invoke-SmokeVerification {
    if (-not ('PastralManagerWindowApi' -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PastralManagerWindowApi
{
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int width, int height, bool repaint);
}
'@
    }

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

        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        $automationRoot = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
        if ($null -eq $automationRoot) {
            Fail 'UI Automation could not resolve the manager root element'
        }

        $historyCondition = New-Object System.Windows.Automation.AndCondition(
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::NameProperty,
                'History'
            )),
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::ListItem
            ))
        )
        $historyItem = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $historyCondition
        )
        if ($null -eq $historyItem) {
            Fail 'UI Automation could not find the History navigation item'
        }
        $selectionPattern = $historyItem.GetCurrentPattern(
            [System.Windows.Automation.SelectionItemPattern]::Pattern
        )
        $selectionPattern.Select()

        $historyDeadline = [DateTime]::UtcNow.AddSeconds(10)
        $requiredHistoryElements = @(
            'History page content',
            'Search clipboard history',
            'History results list',
            'Selected clip details',
            'Synthetic history disclosure'
        )
        foreach ($name in $requiredHistoryElements) {
            $condition = New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::NameProperty,
                $name
            )
            $element = $null
            while ([DateTime]::UtcNow -lt $historyDeadline -and $null -eq $element) {
                Start-Sleep -Milliseconds 150
                $element = $automationRoot.FindFirst(
                    [System.Windows.Automation.TreeScope]::Subtree,
                    $condition
                )
            }
            if ($null -eq $element) {
                Fail "UI Automation could not find History element '$name'"
            }
        }

        $historyListCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            'History results list'
        )
        $historyList = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $historyListCondition
        )
        $listItemCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::ListItem
        )
        $historyRows = $historyList.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            $listItemCondition
        )
        if ($historyRows.Count -ne 6) {
            Fail "History expected six synthetic rows but found $($historyRows.Count)"
        }
        foreach ($row in $historyRows) {
            if ([string]::IsNullOrWhiteSpace($row.Current.Name)) {
                Fail 'History row exposes an empty UI Automation name'
            }
        }
        $firstRow = $historyRows.Item(0)
        if (-not $firstRow.Current.Name.Contains('Build verification passed') -or
            -not $firstRow.Current.Name.Contains('Visual Studio Code')) {
            Fail "History first-row UI Automation name does not contain safe preview/source: '$($firstRow.Current.Name)'"
        }
        $textCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Text
        )
        $firstRowText = @($firstRow.FindAll(
            [System.Windows.Automation.TreeScope]::Subtree,
            $textCondition
        ) | ForEach-Object { $_.Current.Name })
        if ($firstRowText -notcontains 'Build verification passed for the local workspace.' -or
            $firstRowText -notcontains 'Visual Studio Code') {
            Fail ('History first row is missing visible safe preview/source text: ' + ($firstRowText -join ' | '))
        }

        $searchGroupCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            'Search clipboard history'
        )
        $searchGroup = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $searchGroupCondition
        )
        $editCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit
        )
        $searchEdit = $searchGroup.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $editCondition
        )
        if ($null -eq $searchEdit) {
            Fail 'UI Automation could not find the History search edit control'
        }
        $valuePattern = $searchEdit.GetCurrentPattern(
            [System.Windows.Automation.ValuePattern]::Pattern
        )
        $valuePattern.SetValue('Terminal')

        $filteredCountCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            '1 item'
        )
        $filteredCount = $null
        $filterDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([DateTime]::UtcNow -lt $filterDeadline -and $null -eq $filteredCount) {
            Start-Sleep -Milliseconds 150
            $filteredCount = $automationRoot.FindFirst(
                [System.Windows.Automation.TreeScope]::Subtree,
                $filteredCountCondition
            )
        }
        if ($null -eq $filteredCount) {
            Fail 'History UI Automation filtering did not produce the expected one-item result'
        }

        $selectedDetailCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            'Windows Terminal · 8 min ago'
        )
        $selectedDetail = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $selectedDetailCondition
        )
        if ($null -eq $selectedDetail) {
            Fail 'History selection details did not update for the filtered Terminal item'
        }

        if (-not [PastralManagerWindowApi]::MoveWindow($windowHandle, 120, 100, 760, 760, $true)) {
            Fail 'History smoke could not resize the manager to narrow width'
        }
        Start-Sleep -Milliseconds 600

        $narrowHistoryList = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $historyListCondition
        )
        if ($null -eq $narrowHistoryList -or $narrowHistoryList.Current.IsOffscreen) {
            Fail 'History narrow layout did not preserve the results pane before selection'
        }
        $narrowRows = $narrowHistoryList.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            $listItemCondition
        )
        if ($narrowRows.Count -ne 1) {
            Fail "History narrow filtered result expected one row but found $($narrowRows.Count)"
        }
        $narrowSelection = $narrowRows.Item(0).GetCurrentPattern(
            [System.Windows.Automation.SelectionItemPattern]::Pattern
        )
        $narrowSelection.Select()

        $backCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            'Back to history results'
        )
        $backButton = $null
        $backDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([DateTime]::UtcNow -lt $backDeadline -and $null -eq $backButton) {
            Start-Sleep -Milliseconds 100
            $backButton = $automationRoot.FindFirst(
                [System.Windows.Automation.TreeScope]::Subtree,
                $backCondition
            )
        }
        if ($null -eq $backButton -or $backButton.Current.IsOffscreen) {
            Fail 'History narrow selection did not open the one-pane details view with Back action'
        }
        $narrowListAfterSelection = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $historyListCondition
        )
        if ($null -ne $narrowListAfterSelection -and -not $narrowListAfterSelection.Current.IsOffscreen) {
            Fail 'History narrow details view left the results pane visible beside details'
        }
        $backButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Start-Sleep -Milliseconds 250
        $restoredList = $automationRoot.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            $historyListCondition
        )
        if ($null -eq $restoredList -or $restoredList.Current.IsOffscreen) {
            Fail 'History Back action did not restore the results pane'
        }
        if ($valuePattern.Current.Value -ne 'Terminal') {
            Fail 'History Back action did not preserve the active search query'
        }

        $valuePattern.SetValue('no matching Pastral clip')
        $noResultsCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            'No matching clips'
        )
        $noResults = $null
        $noResultsDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while ([DateTime]::UtcNow -lt $noResultsDeadline -and $null -eq $noResults) {
            Start-Sleep -Milliseconds 150
            $noResults = $automationRoot.FindFirst(
                [System.Windows.Automation.TreeScope]::Subtree,
                $noResultsCondition
            )
        }
        if ($null -eq $noResults) {
            Fail 'History UI Automation filtering did not expose the no-results state'
        }

        Write-Host "Manager smoke window handle: $windowHandle"
        Write-Host 'Manager UI Automation History navigation, filtering, selection, and no-results states: PASS'
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
    Remove-Item -LiteralPath $verificationRoot -Recurse -Force -ErrorAction SilentlyContinue
}
