# Fetch VPN backend DLLs for WireSentinel packaging (CI and local release builds).
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

# Pinned upstream versions (update deliberately).
$WIREGUARD_NT_URL = "https://download.wireguard.com/wireguard-nt/wireguard-nt-1.1.zip"
$WIREGUARD_NT_SHA256 = "dceb30a9bc4be48cce0f74160fc88a585a2c2627366e8f846fc6658f9038dace"
$WIREGUARD_WINDOWS_REPO = "https://github.com/WireGuard/wireguard-windows.git"
$WIREGUARD_WINDOWS_REF = "master"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$ResourcesDir = Join-Path $Root "resources"
$CacheDir = Join-Path $Root ".cache" "vpn-resources"
$NtZipPath = Join-Path $CacheDir "wireguard-nt-1.1.zip"
$NtExtractDir = Join-Path $CacheDir "wireguard-nt-extract"

$ArchFolder = if ($Arch -eq "arm64") { "arm64" } else { "amd64" }

function Write-Step([string]$Message) {
    Write-Host "[fetch-vpn] $Message"
}

function Ensure-Directory([string]$Path) {
    if (-not (Test-Path $Path)) {
        New-Item -ItemType Directory -Force -Path $Path | Out-Null
    }
}

function Test-FileSha256([string]$Path, [string]$Expected) {
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

Write-Step "Fetching VPN resources for $Arch"
Install-WireguardNtDll
Install-TunnelDll

foreach ($name in @("tunnel.dll", "wireguard.dll")) {
    $path = Join-Path $ResourcesDir $name
    if (-not (Test-Path $path)) {
        throw "Missing required resource: $path"
    }
    $hash = (Get-FileHash -Path $path -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Step "$name SHA256=$hash"
}

Write-Step "Done"
