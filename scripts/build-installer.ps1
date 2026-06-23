# Unified WireSentinel installer build entry point.
# Usage:
#   .\scripts\build-installer.ps1                    # build binaries + MSI + NSIS (x64)
#   .\scripts\build-installer.ps1 -Arch arm64
#   .\scripts\build-installer.ps1 -SkipBuild         # package only (binaries must exist)
#   .\scripts\build-installer.ps1 -MsiOnly
#   .\scripts\build-installer.ps1 -NsisOnly
#   .\scripts\build-installer.ps1 -SkipDriverBuild   # use pre-staged drivers
#   .\scripts\build-installer.ps1 -SkipDriverSign    # skip test signing (unsigned placeholder .cat)

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [switch]$SkipBuild,
    [switch]$MsiOnly,
    [switch]$NsisOnly,
    [bool]$IncludeDrivers = $true,
    [switch]$SkipDriverBuild,
    [switch]$SkipDriverSign
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$VersionFile = Join-Path $Root "version.json"
if (Test-Path $VersionFile) {
    $Version = (Get-Content -Raw -Path $VersionFile | ConvertFrom-Json).version
} else {
    $Version = "0.1.0"
}

$RustTarget = switch ($Arch) {
    "arm64" { "aarch64-pc-windows-msvc" }
    default { $null }
}

$TargetDir = if ($RustTarget) {
    Join-Path $Root "target\$RustTarget\release"
} else {
    Join-Path $Root "target\release"
}

$Dist = Join-Path $Root "dist"
$ArchLabel = if ($Arch -eq "arm64") { "arm64" } else { "x64" }
$ServiceExe = Join-Path $TargetDir "wire-sentinel-service.exe"
$GuiExe = Join-Path $Root "ui\src-tauri\target\release\wire-sentinel.exe"
if ($RustTarget) {
    $GuiExe = Join-Path $Root "ui\src-tauri\target\$RustTarget\release\wire-sentinel.exe"
}
$TunnelDll = Join-Path $Root "resources\tunnel.dll"
$WireguardDll = Join-Path $Root "resources\wireguard.dll"

$DriverCurrent = Join-Path $Root "installer\staging\drivers\current"
$DriverRequired = @(
    (Join-Path $DriverCurrent "guardian\Guardian.sys"),
    (Join-Path $DriverCurrent "guardian\guardian.inf"),
    (Join-Path $DriverCurrent "ndis\guardian_lwf.sys"),
    (Join-Path $DriverCurrent "ndis\guardian_lwf.inf")
)

function Test-Prerequisites {
    param([string[]]$RequiredFiles)
    foreach ($file in $RequiredFiles) {
        if (-not (Test-Path $file)) {
            throw "Missing build artifact: $file"
        }
    }
}

function Ensure-DriverCatalogPlaceholders {
  param([string]$StageRoot)
  foreach ($pair in @(
      @{ Dir = "guardian"; Cat = "Guardian.cat" },
      @{ Dir = "ndis"; Cat = "guardian_lwf.cat" }
  )) {
      $catPath = Join-Path $StageRoot $pair.Dir $pair.Cat
      if (-not (Test-Path $catPath)) {
          New-Item -ItemType File -Force -Path $catPath | Out-Null
          Write-Host "Created placeholder catalog: $catPath (unsigned local build)"
      }
  }
}

function Stage-InstallerBinaries {
    param(
        [string]$ServiceExe,
        [string]$GuiExe
    )
    $stageDir = Join-Path $Root "installer\staging\bin"
    if (Test-Path $stageDir) {
        Remove-Item -Recurse -Force $stageDir
    }
    New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
    Copy-Item -Force $ServiceExe (Join-Path $stageDir "wire-sentinel-service.exe")
    Copy-Item -Force $GuiExe (Join-Path $stageDir "wire-sentinel.exe")
    Write-Host "Staged installer binaries in $stageDir"
}

    param(
        [switch]$SkipFileRefs,
        [switch]$SkipDriverRefs
    )
    $validate = Join-Path $Root "installer\tests\validate.ps1"
    if (-not (Test-Path $validate)) {
        throw "Missing validation script: $validate"
    }
    $args = @()
    if ($SkipFileRefs) { $args += "-SkipFileRefs" }
    if ($SkipDriverRefs) { $args += "-SkipDriverRefs" }
    & $validate @args
    if ($LASTEXITCODE -ne 0) {
        throw "Installer validation failed (exit $LASTEXITCODE)"
    }
}

if ($IncludeDrivers) {
    $buildDrivers = Join-Path $Root "scripts\build-drivers.ps1"
    $signDrivers = Join-Path $Root "scripts\sign-drivers.ps1"

    if (-not $SkipDriverBuild) {
        Write-Host "Building kernel drivers ($ArchLabel)..."
        & $buildDrivers -Arch $Arch
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    if (-not $SkipDriverSign) {
        & $signDrivers -Arch $Arch
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } else {
        Write-Host "Skipping driver test signing (-SkipDriverSign)"
        Ensure-DriverCatalogPlaceholders -StageRoot $DriverCurrent
    }

    Test-Prerequisites -RequiredFiles $DriverRequired
}

if (-not $SkipBuild) {
    $cargoArgs = @("build", "-p", "core-service", "--release")
    if ($RustTarget) { $cargoArgs += @("--target", $RustTarget) }

    Write-Host "Building wire-sentinel-service (release, $ArchLabel)..."
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "Building Tauri UI (release, $ArchLabel)..."
    Push-Location (Join-Path $Root "ui")
    if (-not (Test-Path "node_modules")) {
        npm install
        if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
    }
    if ($RustTarget) {
        npm run tauri build -- --target $RustTarget
    } else {
        npm run tauri build
    }
    $uiExit = $LASTEXITCODE
    Pop-Location
    if ($uiExit -ne 0) { exit $uiExit }
}

Test-Prerequisites @($ServiceExe, $GuiExe, $TunnelDll, $WireguardDll)
Stage-InstallerBinaries -ServiceExe $ServiceExe -GuiExe $GuiExe
Invoke-Validate

New-Item -ItemType Directory -Force -Path $Dist | Out-Null

$buildMsi = (-not $NsisOnly)
$buildNsis = (-not $MsiOnly)

if ($buildMsi) {
    $msiOut = Join-Path $Dist "WireSentinel-$Version-$ArchLabel.msi"
    Write-Host "Building MSI -> $msiOut"
    $env:WIRESENTINEL_ARCH = $ArchLabel
    wix build -ext WixToolset.Util.wixext (Join-Path $Root "installer\wix\Product.wxs") -o $msiOut
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($buildNsis) {
    Write-Host "Building NSIS setup.exe ($ArchLabel)..."
    $nsisScript = Join-Path $Root "installer\nsis\installer.nsi"
    makensis "/DWIRESENTINEL_VERSION=$Version" "/DWIRESENTINEL_ARCH=$ArchLabel" $nsisScript
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host "Installer build complete. Output: $Dist"
