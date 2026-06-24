# CI helper: install Pester and run static installer e2e checks.
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$GenDir = Join-Path $Root "installer\generated"
New-Item -ItemType Directory -Force -Path $GenDir | Out-Null
@'
; Stub for static CI checks (release builds regenerate via generate-installer-third-party.ps1)
!macro InstallThirdPartyFiles
!macroend
!macro UninstallThirdPartyFiles
!macroend
'@ | Set-Content -Path (Join-Path $GenDir "third-party-files.nsh") -Encoding UTF8

Set-PSRepository PSGallery -InstallationPolicy Trusted
Install-Module Pester -RequiredVersion 5.5.0 -Force -Scope CurrentUser -SkipPublisherCheck

Invoke-Pester -Path (Join-Path $PSScriptRoot "installer-e2e.ps1") -Output Detailed
