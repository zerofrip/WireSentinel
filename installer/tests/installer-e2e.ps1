# Pester static installer e2e checks (no live install required).
# Run: pwsh -File installer/tests/run-pester-ci.ps1

$ErrorActionPreference = "Stop"

BeforeAll {
    $Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $Script:WixFile = Join-Path $Root "installer\wix\Product.wxs"
    $Script:NsisFile = Join-Path $Root "installer\nsis\installer.nsi"
    $Script:WixText = Get-Content -Raw -Path $Script:WixFile
    $Script:NsisText = Get-Content -Raw -Path $Script:NsisFile
}

Describe "WireSentinel installer — clean install" {
    It "WiX registers WireSentinel service with manual start" {
        $Script:WixText | Should -Match 'Name="WireSentinel"'
        $Script:WixText | Should -Match 'Start="demand"'
    }

    It "WiX creates ProgramData directories" {
        $Script:WixText | Should -Match "logs"
        $Script:WixText | Should -Match "tunnels"
        $Script:WixText | Should -Match "transports"
    }

    It "NSIS copies all required binaries" {
        $Script:NsisText | Should -Match "wire-sentinel-service.exe"
        $Script:NsisText | Should -Match "wire-sentinel.exe"
        $Script:NsisText | Should -Match "tunnel.dll"
        $Script:NsisText | Should -Match "wireguard.dll"
    }

    It "NSIS supports silent install" {
        $Script:NsisText | Should -Match "/S"
    }
}

Describe "WireSentinel installer — upgrade" {
    It "WiX defines MajorUpgrade" {
        $Script:WixText | Should -Match "MajorUpgrade"
        $Script:WixText | Should -Match "DowngradeErrorMessage"
    }

    It "WiX uses stable UpgradeCode" {
        $Script:WixText | Should -Match 'UpgradeCode="a1b2c3d4-e5f6-7890-abcd-ef1234567890"'
    }
}

Describe "WireSentinel installer — uninstall" {
    It "WiX removes service on uninstall" {
        $Script:WixText | Should -Match 'Remove="uninstall"'
        $Script:WixText | Should -Match 'Stop="both"'
    }

    It "NSIS stops and deletes service" {
        $Script:NsisText | Should -Match "sc.exe stop WireSentinel"
        $Script:NsisText | Should -Match "sc.exe delete WireSentinel"
    }

    It "NSIS provides uninstaller" {
        $Script:NsisText | Should -Match "WriteUninstaller"
        $Script:NsisText | Should -Match 'Section "Uninstall"'
    }
}

Describe "WireSentinel installer — service recovery" {
    It "WiX configures failure restart actions" {
        $Script:WixText | Should -Match "FirstFailureActionType"
        $Script:WixText | Should -Match "SecondFailureActionType"
        $Script:WixText | Should -Match "ThirdFailureActionType"
        $Script:WixText | Should -Match "RestartServiceDelayInSeconds"
    }

    It "NSIS configures sc failure recovery" {
        $Script:NsisText | Should -Match "sc.exe failure WireSentinel"
        $Script:NsisText | Should -Match "restart/60000"
    }

    It "Both installers depend on Tcpip and Dnscache" {
        $Script:WixText | Should -Match "Tcpip"
        $Script:WixText | Should -Match "Dnscache"
        $Script:NsisText | Should -Match "Tcpip"
        $Script:NsisText | Should -Match "Dnscache"
    }
}

Describe "WireSentinel installer — rollback" {
    It "WiX defines rollback firewall custom action" {
        $Script:WixText | Should -Match "CA_RollbackFirewallRule"
        $Script:WixText | Should -Match 'Execute="rollback"'
    }

    It "WiX schedules rollback before forward firewall action" {
        $Script:WixText | Should -Match 'Action="CA_RollbackFirewallRule" Before="CA_AddFirewallRule"'
    }
}

Describe "WireSentinel installer — kernel drivers" {
    It "WiX exposes optional KernelDriversFeature" {
        $Script:WixText | Should -Match 'Id="KernelDriversFeature"'
        $Script:WixText | Should -Match "DriverPayloadComponents"
    }

    It "WiX schedules kernel driver custom actions" {
        $Script:WixText | Should -Match "CA_InstallKernelDrivers"
        $Script:WixText | Should -Match "CA_UninstallKernelDrivers"
        $Script:WixText | Should -Match '&amp;KernelDriversFeature=3'
    }

    It "NSIS defines optional SecKernelDrivers section" {
        $Script:NsisText | Should -Match "SecKernelDrivers"
        $Script:NsisText | Should -Match "install-kernel-drivers.ps1"
    }

    It "NSIS uninstalls kernel drivers when installed" {
        $Script:NsisText | Should -Match '-Mode Uninstall'
        $Script:NsisText | Should -Match 'KernelDrivers'
    }
}

    It "WiX adds loopback rule on port 8170" {
        $Script:WixText | Should -Match "8170"
        $Script:WixText | Should -Match "127.0.0.1"
        $Script:WixText | Should -Match "WireSentinel API \(loopback\)"
    }

    It "WiX removes firewall rule on uninstall" {
        $Script:WixText | Should -Match "CA_RemoveFirewallRule"
    }

    It "NSIS adds and removes loopback firewall rule" {
        $Script:NsisText | Should -Match 'add rule name="WireSentinel API \(loopback\)"'
        $Script:NsisText | Should -Match 'delete rule name="WireSentinel API \(loopback\)"'
        $Script:NsisText | Should -Match "8170"
        $Script:NsisText | Should -Match "127.0.0.1"
    }
}

