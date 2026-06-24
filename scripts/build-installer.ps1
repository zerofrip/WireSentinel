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
    [bool]$IncludeDrivers = $false,
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

function Get-MakensisExe {
    $cmd = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    foreach ($candidate in @(
            (Join-Path ${env:ProgramFiles(x86)} "NSIS\makensis.exe"),
            (Join-Path $env:ProgramFiles "NSIS\makensis.exe")
        )) {
        if (Test-Path $candidate) { return $candidate }
    }

    throw "makensis.exe not found — install NSIS"
}

function Get-ThirdPartyVersions {
    $versionsFile = Join-Path $Root "installer\third-party-versions.json"
    if (-not (Test-Path $versionsFile)) {
        return $null
    }
    return Get-Content -Raw -Path $versionsFile | ConvertFrom-Json
}

function Write-ThirdPartyNotices {
    param([string]$DestinationDir)

    $noticesTemplate = Join-Path $Root "installer\THIRD_PARTY_NOTICES.txt"
    if (-not (Test-Path $noticesTemplate)) {
        return
    }

    $content = Get-Content -Raw -Path $noticesTemplate
    $versions = Get-ThirdPartyVersions
    if ($versions) {
        if ($versions.'sing-box'.version) {
            $content = $content -replace 'Version: 1\.11\.8', "Version: $($versions.'sing-box'.version)"
        }
        if ($versions.'sing-box'.source_tarball) {
            $content = $content -replace 'https://github.com/SagerNet/sing-box/archive/refs/tags/v1\.11\.8\.tar\.gz', $versions.'sing-box'.source_tarball
        }
        if ($versions.'sing-box'.binary_release) {
            $content = $content -replace 'https://github.com/SagerNet/sing-box/releases/tag/v1\.11\.8', $versions.'sing-box'.binary_release
        }
        if ($versions.windivert.version) {
            $content = $content -replace 'Version: 2\.2\.2-A \(or version bundled at build time\)', "Version: $($versions.windivert.version)"
        }
    }

    Set-Content -Path (Join-Path $DestinationDir "THIRD_PARTY_NOTICES.txt") -Value $content -Encoding UTF8
}

function Stage-ThirdPartyLegal {
    param([string]$DestinationDir)

    Write-ThirdPartyNotices -DestinationDir $DestinationDir

    $licensesSrc = Join-Path $Root "installer\licenses"
    if (Test-Path $licensesSrc) {
        $licensesDst = Join-Path $DestinationDir "licenses"
        New-Item -ItemType Directory -Force -Path $licensesDst | Out-Null
        Copy-Item -Force (Join-Path $licensesSrc "*") $licensesDst
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
    foreach ($pair in @(
            @{ Src = $TunnelDll; Dst = "tunnel.dll" },
            @{ Src = $WireguardDll; Dst = "wireguard.dll" },
            @{ Src = (Join-Path $Root "resources\WinDivert.dll"); Dst = "WinDivert.dll" },
            @{ Src = (Join-Path $Root "resources\WinDivert64.sys"); Dst = "WinDivert64.sys" },
            @{ Src = (Join-Path $Root "resources\sing-box.exe"); Dst = "sing-box.exe" },
            @{ Src = (Join-Path $Root "resources\tor.exe"); Dst = "tor.exe" }
        )) {
        if (Test-Path $pair.Src) {
            Copy-Item -Force $pair.Src (Join-Path $stageDir $pair.Dst)
        }
    }
    Stage-ThirdPartyLegal -DestinationDir $stageDir
    Write-Host "Staged installer binaries in $stageDir"
}

function Invoke-Validate {
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
    Push-Location (Join-Path $Root "installer\wix")
    try {
        wix build -ext WixToolset.Util.wixext Product.wxs -o $msiOut
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }
}

if ($buildNsis) {
    Write-Host "Building NSIS setup.exe ($ArchLabel)..."
    Push-Location (Join-Path $Root "installer\nsis")
    try {
        & (Get-MakensisExe) "/DWIRESENTINEL_VERSION=$Version" "/DWIRESENTINEL_ARCH=$ArchLabel" installer.nsi
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }
}

Write-Host "Installer build complete. Output: $Dist"
