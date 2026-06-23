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
$script:WdkPackagesDir = $null

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

function Initialize-WdkNuGet {
    param([string]$RepoRoot)

    $parent = Split-Path -Parent $RepoRoot
    $wdkDir = Join-Path $RepoRoot "installer\wdk"
    $packagesConfig = Join-Path $wdkDir "packages.config"
    $buildProps = Join-Path $wdkDir "Directory.Build.props"

    if (-not (Test-Path $packagesConfig)) {
        throw "Missing WDK NuGet manifest: $packagesConfig"
    }
    if (-not (Test-Path $buildProps)) {
        throw "Missing WDK MSBuild props: $buildProps"
    }

    Copy-Item -Force $buildProps (Join-Path $parent "Directory.Build.props")
    Copy-Item -Force $packagesConfig (Join-Path $parent "packages.config")

    $script:WdkPackagesDir = Join-Path $parent "packages"
    New-Item -ItemType Directory -Force -Path $script:WdkPackagesDir | Out-Null

    $nuget = Get-Command nuget.exe -ErrorAction SilentlyContinue
    if (-not $nuget) {
        $nugetExe = Join-Path $env:TEMP "nuget.exe"
        if (-not (Test-Path $nugetExe)) {
            Write-Step "Downloading nuget.exe"
            Invoke-WebRequest -Uri "https://dist.nuget.org/win-x86-commandline/latest/nuget.exe" -OutFile $nugetExe
        }
        $nuget = @{ Source = $nugetExe }
    }

    Write-Step "Restoring WDK NuGet packages to $script:WdkPackagesDir"
    & $nuget.Source restore (Join-Path $parent "packages.config") -PackagesDirectory $script:WdkPackagesDir -NonInteractive
    if ($LASTEXITCODE -ne 0) {
        throw "nuget restore failed for WDK packages (exit $LASTEXITCODE)"
    }

    Repair-WdkNuGetToolLayout -PackagesDir $script:WdkPackagesDir
}

function Repair-WdkBuildBinLayout {
    param([string]$PackagesDir)

    # InfVerif loads x86\InfVerif.dll from c\build\<ver>\bin\x86; NuGet WDK may ship x64 only.
    $wdkPackages = Get-ChildItem -Path $PackagesDir -Directory -Filter "Microsoft.Windows.WDK.*" -ErrorAction SilentlyContinue
    foreach ($pkg in $wdkPackages) {
        $buildRoot = Join-Path $pkg.FullName "c\build"
        if (-not (Test-Path $buildRoot)) { continue }

        foreach ($versionDir in Get-ChildItem -Path $buildRoot -Directory) {
            $binRoot = Join-Path $versionDir.FullName "bin"
            $x86Dir = Join-Path $binRoot "x86"
            $x64Dir = Join-Path $binRoot "x64"
            if (-not (Test-Path $x64Dir)) { continue }

            New-Item -ItemType Directory -Force -Path $x86Dir | Out-Null
            $copied = $false
            foreach ($file in Get-ChildItem -Path $x64Dir -File -ErrorAction SilentlyContinue) {
                $dest = Join-Path $x86Dir $file.Name
                if (-not (Test-Path $dest)) {
                    Copy-Item -Force $file.FullName $dest
                    $copied = $true
                }
            }
            if ($copied) {
                Write-Step "Installed build bin x86 tools under $($versionDir.Name) for $($pkg.Name)"
            }
        }
    }
}

function Get-WdkInfToolPath {
    param(
        [string]$PackagesDir,
        [string]$Platform
    )

    $pkgPattern = if ($Platform -eq "ARM64") {
        "Microsoft.Windows.WDK.arm64.*"
    } else {
        "Microsoft.Windows.WDK.x64.*"
    }
    $pkg = Get-ChildItem -Path $PackagesDir -Directory -Filter $pkgPattern -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $pkg) { return $null }

    $binRoot = Join-Path $pkg.FullName "c\bin"
    if (-not (Test-Path $binRoot)) { return $null }

    foreach ($versionDir in (Get-ChildItem -Path $binRoot -Directory | Sort-Object Name -Descending)) {
        foreach ($arch in @("x86", "x64")) {
            $dir = Join-Path $versionDir.FullName $arch
            if (Test-Path (Join-Path $dir "stampinf.exe")) {
                return "$dir\"
            }
        }
    }
    return $null
}

function Repair-WdkNuGetToolLayout {
    param([string]$PackagesDir)

    # NuGet WDK ships stampinf.exe under x64/ARM64 only, but MSBuild's StampInf
    # task looks in WDKBinRoot_x86 (see WindowsDriver.Common.targets).
    $wdkPackages = Get-ChildItem -Path $PackagesDir -Directory -Filter "Microsoft.Windows.WDK.*" -ErrorAction SilentlyContinue
    foreach ($pkg in $wdkPackages) {
        $binRoot = Join-Path $pkg.FullName "c\bin"
        if (-not (Test-Path $binRoot)) { continue }

        foreach ($versionDir in Get-ChildItem -Path $binRoot -Directory) {
            $x86Dir = Join-Path $versionDir.FullName "x86"
            New-Item -ItemType Directory -Force -Path $x86Dir | Out-Null

            $sourceArchDir = $null
            foreach ($arch in @("x64", "ARM64")) {
                $candidate = Join-Path $versionDir.FullName $arch
                if (Test-Path (Join-Path $candidate "stampinf.exe")) {
                    $sourceArchDir = $candidate
                    break
                }
            }
            if (-not $sourceArchDir) { continue }

            $copied = $false
            foreach ($file in Get-ChildItem -Path $sourceArchDir -File -ErrorAction SilentlyContinue) {
                $dest = Join-Path $x86Dir $file.Name
                if (-not (Test-Path $dest)) {
                    Copy-Item -Force $file.FullName $dest
                    $copied = $true
                }
            }
            if ($copied) {
                Write-Step "Installed x86 tool shims under $($versionDir.Name) for $($pkg.Name)"
            }
        }
    }

    Repair-WdkBuildBinLayout -PackagesDir $PackagesDir
}

function Get-MsBuildExe {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $amd64 = & $vswhere -latest -requires Microsoft.Component.MSBuild `
            -find "MSBuild\**\Bin\amd64\MSBuild.exe" |
            Select-Object -First 1
        if ($amd64 -and (Test-Path $amd64)) {
            return $amd64
        }
    }

    $msbuild = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($msbuild) { return $msbuild.Source }
    throw "msbuild.exe not found — install Visual Studio 2022 + WDK"
}

function Invoke-MsBuild {
    param(
        [string]$Project,
        [string]$WorkingDirectory
    )
    $msbuild = Get-MsBuildExe
    Write-Step "Using MSBuild=$msbuild"
    Push-Location $WorkingDirectory
    try {
        $msbuildArgs = @(
            $Project,
            "/p:Configuration=$Configuration",
            "/p:Platform=$MsBuildPlatform",
            "/p:RunInf2Cat=false",
            "/p:RunInfVerif=false",
            "/m"
        )
        if ($script:WdkPackagesDir) {
            $infToolPath = Get-WdkInfToolPath -PackagesDir $script:WdkPackagesDir -Platform $MsBuildPlatform
            if ($infToolPath) {
                Write-Step "Using InfToolPath=$infToolPath"
                $msbuildArgs += "/p:InfToolPath=$infToolPath"
            }
        }
        & $msbuild @msbuildArgs
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

    Initialize-WdkNuGet -RepoRoot $Root

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
