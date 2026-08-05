Set-StrictMode -Version 3.0

function Get-PastralWindowsSdkVersion {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot
    )

    $propsPath = Join-Path $RepositoryRoot 'Directory.Build.props'
    if (-not (Test-Path -LiteralPath $propsPath -PathType Leaf)) {
        throw "Directory.Build.props was not found at $propsPath"
    }

    [xml]$props = [System.IO.File]::ReadAllText($propsPath)
    $versions = @(
        $props.Project.PropertyGroup |
            ForEach-Object { [string]$_.WindowsTargetPlatformVersion } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            Select-Object -Unique
    )
    if ($versions.Count -ne 1 -or $versions[0] -notmatch '^\d+\.\d+\.\d+\.\d+$') {
        throw 'Directory.Build.props must define exactly one four-component WindowsTargetPlatformVersion'
    }
    return $versions[0]
}

function Resolve-PastralWindowsSdkTool {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter()][string]$SdkRoot
    )

    if ([System.IO.Path]::GetFileName($Name) -ne $Name -or
        [System.IO.Path]::GetExtension($Name) -ine '.exe') {
        throw "Windows SDK tool name must be a leaf .exe filename: $Name"
    }

    if ([string]::IsNullOrWhiteSpace($SdkRoot)) {
        try {
            $SdkRoot = [string](Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots').KitsRoot10
        }
        catch {
            throw 'Windows SDK KitsRoot10 could not be read from the registry'
        }
    }
    if ([string]::IsNullOrWhiteSpace($SdkRoot)) {
        throw 'Windows SDK KitsRoot10 is empty'
    }

    $sdkVersion = Get-PastralWindowsSdkVersion -RepositoryRoot $RepositoryRoot
    $tool = Join-Path $SdkRoot ("bin\{0}\x64\{1}" -f $sdkVersion, $Name)
    if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
        throw "$Name x64 for the required Windows SDK $sdkVersion was not found at $tool"
    }
    return $tool
}
