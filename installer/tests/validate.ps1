# Static WiX/NSIS installer validation (file refs, required config strings, XML syntax).
# Run from repo root: .\installer\tests\validate.ps1
# CI (no binaries):   .\installer\tests\validate.ps1 -SkipFileRefs -SkipDriverRefs

param(
    [switch]$SkipFileRefs,
    [switch]$SkipDriverRefs
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))

$WixFile = Join-Path $Root "installer\wix\Product.wxs"
$NsisFile = Join-Path $Root "installer\nsis\installer.nsi"

$DriverSourceFragments = @(
    "installer\staging\drivers\current\guardian",
    "installer\staging\drivers\current\ndis",
    "install-kernel-drivers.ps1"
)

function Test-IsDriverSource {
    param([string]$Source)
    foreach ($fragment in $DriverSourceFragments) {
        if ($Source -match [regex]::Escape($fragment)) {
            return $true
        }
    }
    return $false
}

function Test-XmlSyntax {
    param([string]$Path)
    [xml]$null = Get-Content -Raw -Path $Path
    Write-Host "OK XML syntax: $Path"
}

function Test-WixFileRefs {
    param(
        [string]$Path,
        [switch]$SkipDriverRefs
    )
    [xml]$doc = Get-Content -Raw -Path $Path
    $files = $doc.SelectNodes("//*[local-name()='File']")
    $checked = 0
    foreach ($node in $files) {
        $source = $node.GetAttribute("Source")
        if ([string]::IsNullOrWhiteSpace($source)) {
            throw "WiX File element missing Source in $Path"
        }
        if ($SkipDriverRefs -and (Test-IsDriverSource -Source $source)) {
            continue
        }
        $resolved = Join-Path (Split-Path $Path -Parent) $source
        $resolved = [System.IO.Path]::GetFullPath($resolved)
        if (-not (Test-Path $resolved)) {
            throw "WiX references missing file: $source (resolved: $resolved)"
        }
        $checked++
    }
    Write-Host "OK WiX file references ($checked files checked)"
}

function Test-WixRequiredContent {
    param([string]$Path)
    $text = Get-Content -Raw -Path $Path
    $required = @(
        "CommonAppDataFolder",
        "logs",
        "tunnels",
        "transports",
        'Start="demand"',
        "Tcpip",
        "Dnscache",
        "FirstFailureActionType",
        "RestartServiceDelayInSeconds",
        '"60"',
        "8170",
        "127.0.0.1",
        "WireSentinel API (loopback)",
        "CA_AddFirewallRule",
        "CA_RemoveFirewallRule",
        "CA_RollbackFirewallRule",
        'Execute="rollback"',
        "InstallExecuteSequence",
        "MajorUpgrade",
        "KernelDriversFeature",
        "DriverPayloadComponents",
        "CA_InstallKernelDrivers",
        "CA_UninstallKernelDrivers",
        "install-kernel-drivers.ps1"
    )
    foreach ($token in $required) {
        if ($text -notmatch [regex]::Escape($token)) {
            throw "WiX missing required token: $token"
        }
    }
    Write-Host "OK WiX required configuration tokens"
}

function Test-NsisFileRefs {
    param(
        [string]$Path,
        [switch]$SkipDriverRefs
    )
    $lines = Get-Content -Path $Path
    $fileLines = $lines | Where-Object { $_ -match '^\s*File "' }
    $checked = 0
    foreach ($line in $fileLines) {
        if ($line -match 'File "([^"]+)"') {
            $source = $Matches[1]
            if ($SkipDriverRefs -and (Test-IsDriverSource -Source $source)) {
                continue
            }
            $resolved = Join-Path (Split-Path $Path -Parent) $source
            $resolved = [System.IO.Path]::GetFullPath($resolved)
            if (-not (Test-Path $resolved)) {
                throw "NSIS references missing file: $source (resolved: $resolved)"
            }
            $checked++
        }
    }
    Write-Host "OK NSIS file references ($checked files checked)"
}

function Test-NsisRequiredContent {
    param([string]$Path)
    $text = Get-Content -Raw -Path $Path
    $required = @(
        "COMMONAPPDATA",
        "logs",
        "tunnels",
        "transports",
        "start= demand",
        "Tcpip",
        "Dnscache",
        "8170",
        "127.0.0.1",
        "WireSentinel API (loopback)",
        "/S",
        "Uninstall",
        "WriteUninstaller",
        "SecKernelDrivers",
        "install-kernel-drivers.ps1"
    )
    foreach ($token in $required) {
        if ($text -notmatch [regex]::Escape($token)) {
            throw "NSIS missing required token: $token"
        }
    }
    Write-Host "OK NSIS required configuration tokens"
}

Write-Host "Validating WireSentinel installers..."

if (-not (Test-Path $WixFile)) { throw "Missing WiX file: $WixFile" }
if (-not (Test-Path $NsisFile)) { throw "Missing NSIS file: $NsisFile" }

Test-XmlSyntax -Path $WixFile
Test-WixRequiredContent -Path $WixFile
if (-not $SkipFileRefs) {
    Test-WixFileRefs -Path $WixFile -SkipDriverRefs:$SkipDriverRefs
} else {
    Write-Host "SKIP WiX file references (-SkipFileRefs)"
}

Test-NsisRequiredContent -Path $NsisFile
if (-not $SkipFileRefs) {
    Test-NsisFileRefs -Path $NsisFile -SkipDriverRefs:$SkipDriverRefs
} else {
    Write-Host "SKIP NSIS file references (-SkipFileRefs)"
}

Write-Host "All installer validations passed."
