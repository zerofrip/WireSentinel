# CI helper: install Pester and run static installer e2e checks.
$ErrorActionPreference = "Stop"

Set-PSRepository PSGallery -InstallationPolicy Trusted
Install-Module Pester -RequiredVersion 5.5.0 -Force -Scope CurrentUser -SkipPublisherCheck

Invoke-Pester -Path (Join-Path $PSScriptRoot "installer-e2e.ps1") -Output Detailed
