# Install or uninstall WireSentinel kernel drivers (Guardian WFP + NDIS LWF).
# Used only when enforcement_backend=custom_kernel (test-signed stack).
# Default signed stack (WireGuard NT + WinDivert + sing-box) does not install these drivers.
# Called from MSI/NSIS deferred actions.
# Usage:
#   .\install-kernel-drivers.ps1 -DriverRoot "C:\Program Files\WireSentinel\drivers" -Mode Install
#   .\install-kernel-drivers.ps1 -DriverRoot "C:\Program Files\WireSentinel\drivers" -Mode Uninstall

param(
    [Parameter(Mandatory = $true)]
    [string]$DriverRoot,
    [ValidateSet("Install", "Uninstall")]
    [string]$Mode = "Install"
)

$ErrorActionPreference = "Stop"

$RegKey = "HKLM:\SOFTWARE\WireSentinel\Installer"
$GuardianReg = "GuardianPublishedName"
$NdisReg = "NdisPublishedName"

function Write-Log([string]$Message) {
    Write-Host "[kernel-drivers] $Message"
}

function Invoke-Quiet {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )
    Write-Log "$FilePath $($ArgumentList -join ' ')"
    $proc = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -Wait -PassThru -NoNewWindow
    if ($proc.ExitCode -ne 0) {
        throw "$FilePath failed with exit code $($proc.ExitCode)"
    }
}

function Get-PublishedName {
    param(
        [string]$InfName
    )
    $lines = @(& pnputil.exe /enum-drivers 2>&1)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "Original Name\s*:\s*$([regex]::Escape($InfName))") {
            for ($j = $i; $j -ge 0; $j--) {
                if ($lines[$j] -match "Published Name\s*:\s*(.+)") {
                    return $Matches[1].Trim()
                }
            }
        }
    }
    return $null
}

function Install-Guardian {
    param([string]$Dir)
    $inf = Join-Path $Dir "guardian.inf"
    if (-not (Test-Path $inf)) {
        throw "Missing $inf"
    }
    Invoke-Quiet -FilePath "pnputil.exe" -ArgumentList @("/add-driver", $inf, "/install")
    $published = Get-PublishedName -InfName "guardian.inf"
    if ($published) {
        New-Item -Path $RegKey -Force | Out-Null
        Set-ItemProperty -Path $RegKey -Name $GuardianReg -Value $published
    }
    try {
        Invoke-Quiet -FilePath "sc.exe" -ArgumentList @("start", "WireSentinelGuardian")
    } catch {
        Write-Log "Guardian service start skipped: $_"
    }
}

function Install-Ndis {
    param([string]$Dir)
    $inf = Join-Path $Dir "guardian_lwf.inf"
    if (-not (Test-Path $inf)) {
        throw "Missing $inf"
    }
    Invoke-Quiet -FilePath "pnputil.exe" -ArgumentList @("/add-driver", $inf, "/install")
    $published = Get-PublishedName -InfName "guardian_lwf.inf"
    if ($published) {
        New-Item -Path $RegKey -Force | Out-Null
        Set-ItemProperty -Path $RegKey -Name $NdisReg -Value $published
    }
    $netcfg = Get-Command netcfg.exe -ErrorAction SilentlyContinue
    if ($netcfg) {
        try {
            Invoke-Quiet -FilePath $netcfg.Source -ArgumentList @("-v", "-l", $inf, "-c", "s", "-i", "WireSentinelNdis")
        } catch {
            Write-Log "netcfg bind skipped: $_"
        }
    } else {
        Write-Log "netcfg.exe not found — NDIS filter not bound automatically"
    }
}

function Uninstall-Guardian {
    try { & sc.exe stop WireSentinelGuardian | Out-Null } catch {}
    if (Test-Path $RegKey) {
        $published = (Get-ItemProperty -Path $RegKey -Name $GuardianReg -ErrorAction SilentlyContinue).$GuardianReg
        if ($published) {
            try {
                Invoke-Quiet -FilePath "pnputil.exe" -ArgumentList @("/delete-driver", $published, "/uninstall", "/force")
            } catch {
                Write-Log "Guardian driver removal skipped: $_"
            }
            Remove-ItemProperty -Path $RegKey -Name $GuardianReg -ErrorAction SilentlyContinue
        }
    }
}

function Uninstall-Ndis {
    $guardianDir = Join-Path $DriverRoot "guardian"
    $ndisDir = Join-Path $DriverRoot "ndis"
    $inf = Join-Path $ndisDir "guardian_lwf.inf"
    $netcfg = Get-Command netcfg.exe -ErrorAction SilentlyContinue
    if ($netcfg -and (Test-Path $inf)) {
        try {
            Invoke-Quiet -FilePath $netcfg.Source -ArgumentList @("-u", "-c", "s", "-i", "WireSentinelNdis")
        } catch {
            Write-Log "netcfg unbind skipped: $_"
        }
    }
    if (Test-Path $RegKey) {
        $published = (Get-ItemProperty -Path $RegKey -Name $NdisReg -ErrorAction SilentlyContinue).$NdisReg
        if ($published) {
            try {
                Invoke-Quiet -FilePath "pnputil.exe" -ArgumentList @("/delete-driver", $published, "/uninstall", "/force")
            } catch {
                Write-Log "NDIS driver removal skipped: $_"
            }
            Remove-ItemProperty -Path $RegKey -Name $NdisReg -ErrorAction SilentlyContinue
        }
    }
}

$guardianDir = Join-Path $DriverRoot "guardian"
$ndisDir = Join-Path $DriverRoot "ndis"

if ($Mode -eq "Install") {
    Write-Log "Installing kernel drivers from $DriverRoot"
    Install-Guardian -Dir $guardianDir
    Install-Ndis -Dir $ndisDir
    Write-Log "Install complete"
} else {
    Write-Log "Uninstalling kernel drivers"
    Uninstall-Ndis
    Uninstall-Guardian
    Write-Log "Uninstall complete"
}
