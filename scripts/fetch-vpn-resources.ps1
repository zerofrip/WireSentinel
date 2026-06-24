# Fetch VPN and transport binaries for WireSentinel packaging (CI and local release builds).
# Usage:
#   .\scripts\fetch-vpn-resources.ps1
#   .\scripts\fetch-vpn-resources.ps1 -Arch arm64
#   .\scripts\fetch-vpn-resources.ps1 -WireguardWindowsDir C:\src\wireguard-windows

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [string]$WireguardWindowsDir = ""
)

$ErrorActionPreference = "Stop"

# Pinned upstream versions (update deliberately via installer/third-party-versions.json).
$WIREGUARD_NT_URL = "https://download.wireguard.com/wireguard-nt/wireguard-nt-1.1.zip"
$WIREGUARD_NT_SHA256 = "dceb30a9bc4be48cce0f74160fc88a585a2c2627366e8f846fc6658f9038dace"
$WIREGUARD_WINDOWS_REPO = "https://github.com/WireGuard/wireguard-windows.git"
$WIREGUARD_WINDOWS_REF = "master"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$ResourcesDir = Join-Path $Root "resources"
$CacheDir = Join-Path $Root ".cache" "vpn-resources"
$VersionsFile = Join-Path $Root "installer" "third-party-versions.json"
$NtZipPath = Join-Path $CacheDir "wireguard-nt-1.1.zip"
$NtExtractDir = Join-Path $CacheDir "wireguard-nt-extract"

$ArchFolder = if ($Arch -eq "arm64") { "arm64" } else { "amd64" }
$SingBoxArchKey = if ($Arch -eq "arm64") { "arm64" } else { "x64" }

function Write-Step([string]$Message) {
    Write-Host "[fetch-vpn] $Message"
}

function Ensure-Directory([string]$Path) {
    if (-not (Test-Path $Path)) {
        New-Item -ItemType Directory -Force -Path $Path | Out-Null
    }
}

function Get-ThirdPartyVersions {
    if (-not (Test-Path $VersionsFile)) {
        throw "Missing third-party versions file: $VersionsFile"
    }
    return Get-Content -Raw -Path $VersionsFile | ConvertFrom-Json
}

function Test-FileSha256([string]$Path, [string]$Expected) {
    if ([string]::IsNullOrWhiteSpace($Expected)) {
        return
    }
    $hash = (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $Expected.ToLowerInvariant()) {
        throw "SHA256 mismatch for $Path`: expected $Expected, got $hash"
    }
}

function Invoke-Download([string]$Url, [string]$Destination) {
    Ensure-Directory (Split-Path -Parent $Destination)
    Write-Step "Downloading $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
}

function Install-WireguardNtDll {
    Ensure-Directory $CacheDir
    if (-not (Test-Path $NtZipPath)) {
        Invoke-Download -Url $WIREGUARD_NT_URL -Destination $NtZipPath
    }
    Test-FileSha256 -Path $NtZipPath -Expected $WIREGUARD_NT_SHA256

    if (Test-Path $NtExtractDir) {
        Remove-Item -Recurse -Force $NtExtractDir
    }
    Expand-Archive -Path $NtZipPath -DestinationPath $NtExtractDir -Force

    $dllSrc = Get-ChildItem -Path $NtExtractDir -Recurse -Filter "wireguard.dll" |
        Where-Object { $_.FullName -match [regex]::Escape("\bin\$ArchFolder\") } |
        Select-Object -First 1
    if (-not $dllSrc) {
        throw "wireguard.dll not found under $NtExtractDir\bin\$ArchFolder\"
    }

    Ensure-Directory $ResourcesDir
    $dllDst = Join-Path $ResourcesDir "wireguard.dll"
    Copy-Item -Force $dllSrc.FullName $dllDst
    Write-Step "Installed wireguard.dll ($ArchFolder) -> $dllDst"
}

function Install-TunnelDll {
    if ([string]::IsNullOrWhiteSpace($WireguardWindowsDir)) {
        $WireguardWindowsDir = Join-Path $CacheDir "wireguard-windows"
    }

    if (-not (Test-Path (Join-Path $WireguardWindowsDir ".git"))) {
        Ensure-Directory (Split-Path -Parent $WireguardWindowsDir)
        if (Test-Path $WireguardWindowsDir) {
            Remove-Item -Recurse -Force $WireguardWindowsDir
        }
        Write-Step "Cloning wireguard-windows ($WIREGUARD_WINDOWS_REF)"
        git clone --depth 1 --branch $WIREGUARD_WINDOWS_REF $WIREGUARD_WINDOWS_REPO $WireguardWindowsDir
        if ($LASTEXITCODE -ne 0) {
            throw "git clone wireguard-windows failed with exit code $LASTEXITCODE"
        }
    }

    $embeddableDir = Join-Path $WireguardWindowsDir "embeddable-dll-service"
    if (-not (Test-Path $embeddableDir)) {
        throw "Missing embeddable-dll-service at $embeddableDir"
    }

    $tunnelSrc = Join-Path $embeddableDir $ArchFolder "tunnel.dll"
    if (-not (Test-Path $tunnelSrc)) {
        Write-Step "Building embeddable tunnel.dll (all platforms; may take several minutes on first run)"
        Push-Location $embeddableDir
        try {
            cmd /c build.bat
            if ($LASTEXITCODE -ne 0) {
                throw "embeddable-dll-service build.bat failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    }

    if (-not (Test-Path $tunnelSrc)) {
        throw "tunnel.dll not found at $tunnelSrc after build"
    }

    Ensure-Directory $ResourcesDir
    $tunnelDst = Join-Path $ResourcesDir "tunnel.dll"
    Copy-Item -Force $tunnelSrc $tunnelDst
    Write-Step "Installed tunnel.dll ($ArchFolder) -> $tunnelDst"
}

function Install-WinDivert {
    param($Versions)

    if ($Arch -ne "x64") {
        Write-Step "Skipping WinDivert on $Arch (no official signed ARM64 driver; WFP-only path)"
        return
    }

    $meta = $Versions.windivert
    $zipPath = Join-Path $CacheDir "WinDivert-$($meta.version).zip"
    if (-not (Test-Path $zipPath)) {
        Invoke-Download -Url $meta.download_url -Destination $zipPath
    }
    Test-FileSha256 -Path $zipPath -Expected $meta.sha256

    $extractDir = Join-Path $CacheDir "windivert-extract"
    if (Test-Path $extractDir) {
        Remove-Item -Recurse -Force $extractDir
    }
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $folder = $meta.folder
    $dllSrc = Get-ChildItem -Path $extractDir -Recurse -Filter "WinDivert.dll" |
        Where-Object { $_.FullName -match [regex]::Escape("\$folder\") } |
        Select-Object -First 1
    $sysSrc = Get-ChildItem -Path $extractDir -Recurse -Filter "WinDivert64.sys" |
        Where-Object { $_.FullName -match [regex]::Escape("\$folder\") } |
        Select-Object -First 1
    if (-not $dllSrc -or -not $sysSrc) {
        throw "WinDivert binaries not found under $extractDir\$folder\"
    }

    Ensure-Directory $ResourcesDir
    Copy-Item -Force $dllSrc.FullName (Join-Path $ResourcesDir "WinDivert.dll")
    Copy-Item -Force $sysSrc.FullName (Join-Path $ResourcesDir "WinDivert64.sys")
    Write-Step "Installed WinDivert.dll + WinDivert64.sys ($folder) -> $ResourcesDir"
}

function Install-SingBox {
    param($Versions)

    $meta = $Versions.'sing-box'
    $asset = $meta.assets.$SingBoxArchKey
    if (-not $asset) {
        throw "sing-box asset not defined for architecture key $SingBoxArchKey"
    }

    $zipPath = Join-Path $CacheDir $asset.file
    if (-not (Test-Path $zipPath)) {
        $url = "https://github.com/SagerNet/sing-box/releases/download/v$($meta.version)/$($asset.file)"
        Invoke-Download -Url $url -Destination $zipPath
    }
    Test-FileSha256 -Path $zipPath -Expected $asset.sha256

    $extractDir = Join-Path $CacheDir "sing-box-extract-$SingBoxArchKey"
    if (Test-Path $extractDir) {
        Remove-Item -Recurse -Force $extractDir
    }
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $exeSrc = Get-ChildItem -Path $extractDir -Recurse -Filter "sing-box.exe" | Select-Object -First 1
    if (-not $exeSrc) {
        throw "sing-box.exe not found in $extractDir"
    }

    Ensure-Directory $ResourcesDir
    Copy-Item -Force $exeSrc.FullName (Join-Path $ResourcesDir "sing-box.exe")
    Write-Step "Installed sing-box.exe ($SingBoxArchKey) -> $ResourcesDir"
}

function Install-Tor {
    param($Versions)

    $meta = $Versions.tor
    $archiveName = Split-Path $meta.download_url -Leaf
    $archivePath = Join-Path $CacheDir $archiveName
    if (-not (Test-Path $archivePath)) {
        Invoke-Download -Url $meta.download_url -Destination $archivePath
    }
    Test-FileSha256 -Path $archivePath -Expected $meta.sha256

    $extractDir = Join-Path $CacheDir "tor-extract"
    if (Test-Path $extractDir) {
        Remove-Item -Recurse -Force $extractDir
    }
    Ensure-Directory $extractDir
    tar -xzf $archivePath -C $extractDir

    $exeSrc = Get-ChildItem -Path $extractDir -Recurse -Filter "tor.exe" |
        Where-Object { $_.Directory.Name -eq "tor" } |
        Select-Object -First 1
    if (-not $exeSrc) {
        $exeSrc = Get-ChildItem -Path $extractDir -Recurse -Filter "tor.exe" | Select-Object -First 1
    }
    if (-not $exeSrc) {
        throw "tor.exe not found in $extractDir"
    }

    Ensure-Directory $ResourcesDir
    Copy-Item -Force $exeSrc.FullName (Join-Path $ResourcesDir "tor.exe")
    if ($Arch -eq "arm64") {
        Write-Step "Installed tor.exe (x86_64 expert bundle for WoA emulation) -> $ResourcesDir"
    } else {
        Write-Step "Installed tor.exe -> $ResourcesDir"
    }
}

Write-Step "Fetching packaging resources for $Arch"
$versions = Get-ThirdPartyVersions
Install-WireguardNtDll
Install-TunnelDll
Install-WinDivert -Versions $versions
Install-SingBox -Versions $versions
Install-Tor -Versions $versions

$required = @("tunnel.dll", "wireguard.dll", "sing-box.exe", "tor.exe")
if ($Arch -eq "x64") {
    $required += @("WinDivert.dll", "WinDivert64.sys")
}

foreach ($name in $required) {
    $path = Join-Path $ResourcesDir $name
    if (-not (Test-Path $path)) {
        throw "Missing required resource: $path"
    }
    $hash = (Get-FileHash -Path $path -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Step "$name SHA256=$hash"
}

Write-Step "Done"
