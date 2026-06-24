# WireSentinel release packaging: ZIP + MSI + NSIS + manifest.json (SHA256).
# Usage:
#   .\scripts\release-builder.ps1
#   .\scripts\release-builder.ps1 -Arch arm64
#   .\scripts\release-builder.ps1 -SkipBuild

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [switch]$SkipBuild,
    [switch]$SkipDriverSign
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$VersionFile = Join-Path $Root "version.json"
if (-not (Test-Path $VersionFile)) {
    throw "Missing version.json at repo root"
}
$VersionMeta = Get-Content -Raw -Path $VersionFile | ConvertFrom-Json
$Version = $VersionMeta.version
$Channel = $VersionMeta.channel

$Dist = Join-Path $Root "dist"
$ReleaseDir = Join-Path $Dist "release"
$ArchLabel = if ($Arch -eq "arm64") { "arm64" } else { "x64" }
$ReleaseArchDir = Join-Path $ReleaseDir $ArchLabel

function Get-Sha256 {
    param([string]$Path)
    $hash = Get-FileHash -Path $Path -Algorithm SHA256
    return $hash.Hash.ToLowerInvariant()
}

function New-ZipPackage {
    param(
        [string]$SourceDir,
        [string]$ZipPath
    )
    if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
    Compress-Archive -Path (Join-Path $SourceDir "*") -DestinationPath $ZipPath -CompressionLevel Optimal
}

function Add-DriverManifestEntries {
    param(
        [string]$DriversRoot,
        [ref]$Entries
    )
    if (-not (Test-Path $DriversRoot)) { return }
    $files = Get-ChildItem -Path $DriversRoot -Recurse -File -ErrorAction SilentlyContinue
    foreach ($file in $files) {
        $relative = $file.FullName.Substring($DriversRoot.Length).TrimStart("\", "/")
        $Entries.Value += [ordered]@{
            path       = "drivers/$relative".Replace("\", "/")
            sha256     = Get-Sha256 -Path $file.FullName
            size_bytes = $file.Length
        }
    }
}

Write-Host "WireSentinel release builder v$Version ($ArchLabel, channel=$Channel)"

$BuildArgs = @{
    Arch = $Arch
}
if ($SkipBuild) { $BuildArgs.SkipBuild = $true }
if ($SkipDriverSign) { $BuildArgs.SkipDriverSign = $true }
& (Join-Path $Root "scripts\build-installer.ps1") @BuildArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force -Path $ReleaseArchDir | Out-Null

$MsiName = "WireSentinel-$Version-$ArchLabel.msi"
$NsisName = "WireSentinel-$Version-$ArchLabel-setup.exe"
$ZipName = "WireSentinel-$Version-$ArchLabel.zip"

$MsiSrc = Join-Path $Dist "WireSentinel-$Version-$ArchLabel.msi"
$NsisSrc = Join-Path $Dist "WireSentinel-$Version-$ArchLabel-setup.exe"

if (-not (Test-Path $MsiSrc)) {
    $MsiSrcFallback = Join-Path $Dist "WireSentinel-$Version.msi"
    if (Test-Path $MsiSrcFallback) {
        Copy-Item -Force $MsiSrcFallback (Join-Path $ReleaseArchDir $MsiName)
        $MsiSrc = Join-Path $ReleaseArchDir $MsiName
    } else {
        throw "MSI artifact not found: $MsiSrc"
    }
} else {
    Copy-Item -Force $MsiSrc (Join-Path $ReleaseArchDir $MsiName)
    $MsiSrc = Join-Path $ReleaseArchDir $MsiName
}

if (-not (Test-Path $NsisSrc)) {
    $NsisSrcFallback = Join-Path $Dist "WireSentinel-$Version-setup.exe"
    if (Test-Path $NsisSrcFallback) {
        Copy-Item -Force $NsisSrcFallback (Join-Path $ReleaseArchDir $NsisName)
        $NsisSrc = Join-Path $ReleaseArchDir $NsisName
    } else {
        throw "NSIS artifact not found: $NsisSrc"
    }
} else {
    Copy-Item -Force $NsisSrc (Join-Path $ReleaseArchDir $NsisName)
    $NsisSrc = Join-Path $ReleaseArchDir $NsisName
}

$Staging = Join-Path $ReleaseArchDir "zip-staging"
if (Test-Path $Staging) { Remove-Item -Recurse -Force $Staging }
New-Item -ItemType Directory -Force -Path $Staging | Out-Null

$ServiceExe = Join-Path $Root "target\release\wire-sentinel-service.exe"
$GuiExe = Join-Path $Root "ui\src-tauri\target\release\wire-sentinel.exe"
if ($Arch -eq "arm64") {
    $ServiceExe = Join-Path $Root "target\aarch64-pc-windows-msvc\release\wire-sentinel-service.exe"
    $GuiExe = Join-Path $Root "ui\src-tauri\target\aarch64-pc-windows-msvc\release\wire-sentinel.exe"
}

Copy-Item -Force $ServiceExe $Staging
Copy-Item -Force $GuiExe $Staging
Copy-Item -Force (Join-Path $Root "resources\tunnel.dll") $Staging
Copy-Item -Force (Join-Path $Root "resources\wireguard.dll") $Staging
Copy-Item -Force $VersionFile $Staging

$requiredResources = @("sing-box.exe", "tor.exe")
if ($Arch -eq "x64") {
    $requiredResources += @("WinDivert.dll", "WinDivert64.sys")
}
foreach ($name in $requiredResources) {
    $src = Join-Path $Root "resources\$name"
    if (-not (Test-Path $src)) {
        throw "Missing required release resource: $src"
    }
    Copy-Item -Force $src $Staging
}

$stagedBin = Join-Path $Root "installer\staging\bin"
$notices = Join-Path $stagedBin "THIRD_PARTY_NOTICES.txt"
if (Test-Path $notices) {
    Copy-Item -Force $notices (Join-Path $Staging "THIRD_PARTY_NOTICES.txt")
} else {
    $noticesFallback = Join-Path $Root "installer\THIRD_PARTY_NOTICES.txt"
    if (Test-Path $noticesFallback) {
        Copy-Item -Force $noticesFallback (Join-Path $Staging "THIRD_PARTY_NOTICES.txt")
    }
}
$licensesSrc = Join-Path $stagedBin "licenses"
if (-not (Test-Path $licensesSrc)) {
    $licensesSrc = Join-Path $Root "installer\licenses"
}
if (Test-Path $licensesSrc) {
    $licensesDst = Join-Path $Staging "licenses"
    New-Item -ItemType Directory -Force -Path $licensesDst | Out-Null
    Copy-Item -Force (Join-Path $licensesSrc "*") $licensesDst
}

$DriverStage = Join-Path $Root "installer\staging\drivers\$ArchLabel"
if (Test-Path $DriverStage) {
    Copy-Item -Recurse -Force $DriverStage (Join-Path $Staging "drivers")
}

$ZipPath = Join-Path $ReleaseArchDir $ZipName
New-ZipPackage -SourceDir $Staging -ZipPath $ZipPath
Remove-Item -Recurse -Force $Staging

$artifacts = @(
    @{ path = $MsiName; file = $MsiSrc },
    @{ path = $NsisName; file = $NsisSrc },
    @{ path = $ZipName; file = $ZipPath }
)

$manifestEntries = @()
foreach ($item in $artifacts) {
    $manifestEntries += [ordered]@{
        path       = $item.path
        sha256     = Get-Sha256 -Path $item.file
        size_bytes = (Get-Item $item.file).Length
    }
}

if (Test-Path $DriverStage) {
    Add-DriverManifestEntries -DriversRoot $DriverStage -Entries ([ref]$manifestEntries)
}

$manifest = [ordered]@{
    version    = $Version
    channel    = $Channel
    build_date = (Get-Date).ToUniversalTime().ToString("o")
    arch       = $ArchLabel
    artifacts  = $manifestEntries
}

$ManifestPath = Join-Path $ReleaseArchDir "manifest.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -Path $ManifestPath -Encoding UTF8

Write-Host "Release artifacts:"
foreach ($entry in $manifestEntries) {
    Write-Host "  $($entry.path)  sha256=$($entry.sha256)  size=$($entry.size_bytes)"
}
Write-Host "Manifest: $ManifestPath"
Write-Host "Release packaging complete."
