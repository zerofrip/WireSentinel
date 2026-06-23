# Sign staged kernel driver packages with a self-signed test code-signing certificate.
# Usage:
#   .\scripts\sign-drivers.ps1 -Arch x64
#   .\scripts\sign-drivers.ps1 -SkipSign
#
# Target machines must enable testsigning: bcdedit /set testsigning on

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [switch]$SkipSign
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$ArchLabel = if ($Arch -eq "arm64") { "arm64" } else { "x64" }
$StageRoot = Join-Path $Root "installer\staging\drivers\$ArchLabel"

# Dev-only password for the cached test PFX (not a production secret).
$TestPfxPassword = "wiresentinel-test"
$TestCertSubject = "CN=WireSentinel Test Code Signing"
$TestPfxCacheDir = Join-Path $Root ".cache\test-signing"
$TestPfxPath = Join-Path $TestPfxCacheDir "wiresentinel-test.pfx"

function Write-Step([string]$Message) {
    Write-Host "[sign-drivers] $Message"
}

function Get-WdkPackagesDir {
    $parent = Split-Path -Parent $Root
    $packagesDir = Join-Path $parent "packages"
    if (Test-Path $packagesDir) { return $packagesDir }
    return $null
}

function Repair-WdkInfToolLayout {
    param([string]$PackagesDir)

    if (-not $PackagesDir -or -not (Test-Path $PackagesDir)) { return }

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

            foreach ($file in Get-ChildItem -Path $sourceArchDir -File -ErrorAction SilentlyContinue) {
                $dest = Join-Path $x86Dir $file.Name
                if (-not (Test-Path $dest)) {
                    Copy-Item -Force $file.FullName $dest
                }
            }
        }
    }
}

function Get-WdkInfToolDir {
    param(
        [string]$PackagesDir,
        [string]$ArchLabel
    )

    if (-not $PackagesDir) { return $null }

    $pkgPattern = if ($ArchLabel -eq "arm64") {
        "Microsoft.Windows.WDK.arm64.*"
    } else {
        "Microsoft.Windows.WDK.x64.*"
    }
    $pkg = Get-ChildItem -Path $PackagesDir -Directory -Filter $pkgPattern -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if (-not $pkg) { return $null }

    $binRoot = Join-Path $pkg.FullName "c\bin"
    if (-not (Test-Path $binRoot)) { return $null }

    foreach ($versionDir in (Get-ChildItem -Path $binRoot -Directory | Sort-Object Name -Descending)) {
        foreach ($arch in @("x86", "x64")) {
            $dir = Join-Path $versionDir.FullName $arch
            if (Test-Path (Join-Path $dir "inf2cat.exe")) {
                return $dir
            }
        }
    }
    return $null
}

function Get-ToolPath {
    param(
        [string]$Name,
        [string]$ArchLabel = "x64"
    )

    $packagesDir = Get-WdkPackagesDir
    if ($packagesDir) {
        Repair-WdkInfToolLayout -PackagesDir $packagesDir
        $toolDir = Get-WdkInfToolDir -PackagesDir $packagesDir -ArchLabel $ArchLabel
        if ($toolDir) {
            $candidate = Join-Path $toolDir $Name
            if (Test-Path $candidate) { return $candidate }
        }
    }

    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    if ($packagesDir) {
        $match = Get-ChildItem -Path $packagesDir -Recurse -Filter $Name -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\c\\bin\\' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($match) { return $match.FullName }
    }

    $kits = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    if (Test-Path $kits) {
        $match = Get-ChildItem -Path $kits -Recurse -Filter $Name -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($match) { return $match.FullName }
    }
    throw "$Name not found — install Windows SDK / WDK"
}

function Get-OrCreateTestPfx {
    if (Test-Path $TestPfxPath) {
        Write-Step "Using cached test PFX: $TestPfxPath"
        return @{
            Path     = $TestPfxPath
            Password = $TestPfxPassword
        }
    }

    New-Item -ItemType Directory -Force -Path $TestPfxCacheDir | Out-Null
    Write-Step "Creating test code-signing certificate ($TestCertSubject)"

    $cert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $TestCertSubject `
        -KeyAlgorithm RSA `
        -KeyLength 2048 `
        -CertStoreLocation Cert:\CurrentUser\My `
        -NotAfter (Get-Date).AddYears(10)

    $securePassword = ConvertTo-SecureString -String $TestPfxPassword -Force -AsPlainText
    Export-PfxCertificate -Cert $cert -FilePath $TestPfxPath -Password $securePassword | Out-Null
    Write-Step "Exported test PFX to $TestPfxPath"

    return @{
        Path     = $TestPfxPath
        Password = $TestPfxPassword
    }
}

function Invoke-Inf2Cat {
    param(
        [string]$DriverDir,
        [string[]]$OsVersions,
        [string]$ArchLabel
    )
    $inf2cat = Get-ToolPath -Name "inf2cat.exe" -ArchLabel $ArchLabel
    $toolDir = Split-Path -Parent $inf2cat
    $previousPath = $env:PATH
  $env:PATH = "$toolDir;$previousPath"
    try {
        $args = @("/driver:$DriverDir", "/verbose")
        foreach ($os in $OsVersions) {
            $args += "/os:$os"
        }
        Write-Step "inf2cat $($args -join ' ')"
        & $inf2cat @args
        if ($LASTEXITCODE -ne 0) {
            throw "inf2cat failed for $DriverDir (exit $LASTEXITCODE)"
        }
    } finally {
        $env:PATH = $previousPath
    }
}

function Invoke-SignFile {
    param(
        [string]$Path,
        [string]$PfxPath,
        [string]$PfxPassword,
        [string]$ArchLabel
    )
    $signtool = Get-ToolPath -Name "signtool.exe" -ArchLabel $ArchLabel
    # No timestamp — test certificates typically fail with public TSA endpoints.
    & $signtool sign /fd SHA256 /f $PfxPath /p $PfxPassword $Path
    if ($LASTEXITCODE -ne 0) {
        throw "signtool failed for $Path (exit $LASTEXITCODE)"
    }
    Write-Step "Test-signed $Path"
}

function Sign-DriverPackage {
    param(
        [string]$PackageDir,
        [string]$SysName,
        [string]$CatName,
        [string[]]$OsVersions,
        [string]$PfxPath,
        [string]$PfxPassword,
        [string]$ArchLabel
    )
    if (-not (Test-Path $PackageDir)) {
        throw "Driver package directory not found: $PackageDir"
    }
    Invoke-Inf2Cat -DriverDir $PackageDir -OsVersions $OsVersions -ArchLabel $ArchLabel
    Invoke-SignFile -Path (Join-Path $PackageDir $SysName) -PfxPath $PfxPath -PfxPassword $PfxPassword -ArchLabel $ArchLabel
    $cat = Join-Path $PackageDir $CatName
    if (Test-Path $cat) {
        Invoke-SignFile -Path $cat -PfxPath $PfxPath -PfxPassword $PfxPassword -ArchLabel $ArchLabel
    }
}

function Sync-CurrentStaging {
    $current = Join-Path $Root "installer\staging\drivers\current"
    if (Test-Path $current) {
        Remove-Item -Recurse -Force $current
    }
    New-Item -ItemType Directory -Force -Path $current | Out-Null
    Copy-Item -Recurse -Force (Join-Path $StageRoot "guardian") (Join-Path $current "guardian")
    Copy-Item -Recurse -Force (Join-Path $StageRoot "ndis") (Join-Path $current "ndis")
}

if ($SkipSign) {
    Write-Step "Skipping driver signing (-SkipSign)"
    exit 0
}

$testPfx = Get-OrCreateTestPfx

$osList = if ($Arch -eq "arm64") {
    @("10_NI_ARM64", "10_VB_ARM64")
} else {
    @("10_NI_X64", "10_VB_X64")
}

Sign-DriverPackage `
    -PackageDir (Join-Path $StageRoot "guardian") `
    -SysName "Guardian.sys" `
    -CatName "Guardian.cat" `
    -OsVersions $osList `
    -PfxPath $testPfx.Path `
    -PfxPassword $testPfx.Password `
    -ArchLabel $ArchLabel

Sign-DriverPackage `
    -PackageDir (Join-Path $StageRoot "ndis") `
    -SysName "guardian_lwf.sys" `
    -CatName "guardian_lwf.cat" `
    -OsVersions $osList `
    -PfxPath $testPfx.Path `
    -PfxPassword $testPfx.Password `
    -ArchLabel $ArchLabel

Sync-CurrentStaging

Write-Step "Done ($ArchLabel) — install target needs: bcdedit /set testsigning on"
