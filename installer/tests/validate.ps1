# Static WiX/NSIS installer validation (file refs, required config strings, XML syntax).
# Run from repo root: .\installer\tests\validate.ps1
# CI (no binaries):   .\installer\tests\validate.ps1 -SkipFileRefs -SkipDriverRefs
# Local smoke compile:  .\installer\tests\validate.ps1 -CompileWixSmoke

param(
    [switch]$SkipFileRefs,
    [switch]$SkipDriverRefs,
    [switch]$CompileWixSmoke
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))

$WixFile = Join-Path $Root "installer\wix\Product.wxs"
$NsisFile = Join-Path $Root "installer\nsis\installer.nsi"
$ThirdPartyWixFile = Join-Path $Root "installer\generated\third-party.wxs"
$StageBinDir = Join-Path $Root "installer\staging\bin"

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

function Test-WixSmokeCompile {
    param([string]$ArchLabel = "x64")
    if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
        Write-Host "SKIP WiX smoke compile (wix CLI not found)"
        return
    }
    if (-not (Test-Path $StageBinDir)) {
        throw "WiX smoke compile requires staged binaries: $StageBinDir"
    }
    if (-not (Test-Path $ThirdPartyWixFile)) {
        throw "WiX smoke compile requires generated fragment: $ThirdPartyWixFile"
    }

    $smokeOut = Join-Path $env:TEMP "wiresentinel-wix-smoke-$ArchLabel.msi"
    if (Test-Path $smokeOut) {
        Remove-Item -Force $smokeOut
    }

    $wixDir = Join-Path $Root "installer\wix"
    Push-Location $wixDir
    try {
        wix build -ext WixToolset.Util.wixext -d WIRESENTINEL_ARCH=$ArchLabel Product.wxs ..\generated\third-party.wxs -o $smokeOut
        if ($LASTEXITCODE -ne 0) {
            throw "WiX smoke compile failed (exit $LASTEXITCODE)"
        }
        Write-Host "OK WiX smoke compile ($ArchLabel)"
    } finally {
        Pop-Location
        if (Test-Path $smokeOut) {
            Remove-Item -Force $smokeOut
        }
    }
}

Write-Host "Validating WireSentinel installers..."

if (-not (Test-Path $WixFile)) { throw "Missing WiX file: $WixFile" }
if (-not (Test-Path $NsisFile)) { throw "Missing NSIS file: $NsisFile" }

Test-XmlSyntax -Path $WixFile
Test-WixRequiredContent -Path $WixFile
if (-not $SkipFileRefs) {
    Test-WixFileRefs -Path $WixFile -SkipDriverRefs:$SkipDriverRefs
    if (Test-Path $ThirdPartyWixFile) {
        Test-WixFileRefs -Path $ThirdPartyWixFile -SkipDriverRefs:$SkipDriverRefs
    } else {
        Write-Host "SKIP generated third-party WiX fragment (not built yet)"
    }
} else {
    Write-Host "SKIP WiX file references (-SkipFileRefs)"
}

Test-NsisRequiredContent -Path $NsisFile
if (-not $SkipFileRefs) {
    Test-NsisFileRefs -Path $NsisFile -SkipDriverRefs:$SkipDriverRefs
} else {
    Write-Host "SKIP NSIS file references (-SkipFileRefs)"
}

if ($CompileWixSmoke) {
    if ((Test-Path $StageBinDir) -and (Test-Path $ThirdPartyWixFile)) {
        Test-WixSmokeCompile -ArchLabel "x64"
    } else {
        Write-Host "SKIP WiX smoke compile (staging/bin or generated/third-party.wxs missing)"
    }
}

Write-Host "All installer validations passed."
