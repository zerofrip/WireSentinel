# Build Guardian (WFP) and NDIS LWF kernel drivers and stage for the installer.
# Usage:
#   .\scripts\build-drivers.ps1
#   .\scripts\build-drivers.ps1 -Arch arm64
#   .\scripts\build-drivers.ps1 -SkipBuild

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [string]$Configuration = "Release",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Parent = Split-Path -Parent $Root

$KernelRoot = Join-Path $Parent "WireSentinel-Kernel"
$NdisRoot = Join-Path $Parent "WireSentinel-Ndis"

$MsBuildPlatform = if ($Arch -eq "arm64") { "ARM64" } else { "x64" }
$ArchLabel = if ($Arch -eq "arm64") { "arm64" } else { "x64" }

$StageRoot = Join-Path $Root "installer\staging\drivers\$ArchLabel"
$GuardianStage = Join-Path $StageRoot "guardian"
$NdisStage = Join-Path $StageRoot "ndis"
$CurrentLink = Join-Path $Root "installer\staging\drivers\current"

function Write-Step([string]$Message) {
    Write-Host "[build-drivers] $Message"
}

function Get-VersionMeta {
    $versionFile = Join-Path $Root "version.json"
    if (-not (Test-Path $versionFile)) {
        return @{ version = "0.1.0" }
    }
    return Get-Content -Raw -Path $versionFile | ConvertFrom-Json
}

function Set-InfDriverVer {
    param(
        [string]$InfPath,
        [string]$Version
    )
    if (-not (Test-Path $InfPath)) {
        throw "INF not found: $InfPath"
    }
    $parts = $Version.Split(".")
    while ($parts.Count -lt 4) { $parts += "0" }
    $driverVer = (Get-Date -Format "MM/dd/yyyy") + "," + ($parts[0..3] -join ".")
    $content = Get-Content -Raw -Path $InfPath
    $content = [regex]::Replace(
        $content,
        "DriverVer\s*=\s*[^\r\n]+",
        "DriverVer   = $driverVer"
    )
    Set-Content -Path $InfPath -Value $content -Encoding ASCII -NoNewline
}

function Invoke-MsBuild {
    param(
        [string]$Project,
        [string]$WorkingDirectory
    )
    $msbuild = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if (-not $msbuild) {
        throw "msbuild.exe not found — install Visual Studio 2022 + WDK"
    }
    Push-Location $WorkingDirectory
    try {
        & msbuild.exe $Project /p:Configuration=$Configuration /p:Platform=$MsBuildPlatform /m
        if ($LASTEXITCODE -ne 0) {
            throw "msbuild failed for $Project (exit $LASTEXITCODE)"
        }
    } finally {
        Pop-Location
    }
}

function Copy-DriverPayload {
    param(
        [string]$SourceDir,
        [string]$DestDir,
        [string]$SysName,
        [string]$InfSource,
        [string]$InfDestName,
        [string]$CatName
    )
    if (Test-Path $DestDir) {
        Remove-Item -Recurse -Force $DestDir
    }
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null

    $sysSrc = Join-Path $SourceDir $SysName
    if (-not (Test-Path $sysSrc)) {
        throw "Driver binary not found: $sysSrc"
    }
    Copy-Item -Force $sysSrc (Join-Path $DestDir $SysName)
    Copy-Item -Force $InfSource (Join-Path $DestDir $InfDestName)

    $version = (Get-VersionMeta).version
    Set-InfDriverVer -InfPath (Join-Path $DestDir $InfDestName) -Version $version

    if (Test-Path (Join-Path $SourceDir $CatName)) {
        Copy-Item -Force (Join-Path $SourceDir $CatName) (Join-Path $DestDir $CatName)
    }
}

if (-not $SkipBuild) {
    if (-not (Test-Path $KernelRoot)) {
        throw "WireSentinel-Kernel not found at $KernelRoot"
    }
    if (-not (Test-Path $NdisRoot)) {
        throw "WireSentinel-Ndis not found at $NdisRoot"
    }

    Write-Step "Building Guardian.sys ($Configuration|$MsBuildPlatform)"
    Invoke-MsBuild -Project "guardian\guardian.sln" -WorkingDirectory $KernelRoot

    Write-Step "Building guardian_lwf.sys ($Configuration|$MsBuildPlatform)"
    Invoke-MsBuild -Project "ndis-filter\guardian_lwf.vcxproj" -WorkingDirectory $NdisRoot
}

$guardianOut = Join-Path $KernelRoot "guardian\bin\$MsBuildPlatform\$Configuration"
$ndisOut = Join-Path $NdisRoot "ndis-filter\$MsBuildPlatform\$Configuration"

Write-Step "Staging drivers to $StageRoot"
Copy-DriverPayload `
    -SourceDir $guardianOut `
    -DestDir $GuardianStage `
    -SysName "Guardian.sys" `
    -InfSource (Join-Path $KernelRoot "guardian\guardian.inf") `
    -InfDestName "guardian.inf" `
    -CatName "Guardian.cat"

Copy-DriverPayload `
    -SourceDir $ndisOut `
    -DestDir $NdisStage `
    -SysName "guardian_lwf.sys" `
    -InfSource (Join-Path $NdisRoot "ndis-filter\guardian_lwf.inf") `
    -InfDestName "guardian_lwf.inf" `
    -CatName "guardian_lwf.cat"

if (Test-Path $CurrentLink) {
    Remove-Item -Recurse -Force $CurrentLink
}
New-Item -ItemType Directory -Force -Path $CurrentLink | Out-Null
Copy-Item -Recurse -Force (Join-Path $StageRoot "guardian") (Join-Path $CurrentLink "guardian")
Copy-Item -Recurse -Force (Join-Path $StageRoot "ndis") (Join-Path $CurrentLink "ndis")

Write-Step "Done ($ArchLabel)"
