# Thin wrapper for uninstall — delegates to install-kernel-drivers.ps1 -Mode Uninstall.
param(
    [Parameter(Mandatory = $true)]
    [string]$DriverRoot
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
& (Join-Path $scriptDir "install-kernel-drivers.ps1") -DriverRoot $DriverRoot -Mode Uninstall
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
