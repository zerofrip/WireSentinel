; WireSentinel NSIS installer
; Build: makensis installer/nsis/installer.nsi
; Silent: WireSentinel-0.1.0-setup.exe /S

!include "LogicLib.nsh"
!include "Sections.nsh"
!include "..\generated\third-party-files.nsh"

!define PRODUCT_NAME "WireSentinel"
!define PRODUCT_VERSION "0.1.0"
!define PRODUCT_PUBLISHER "WireSentinel Contributors"
!define INSTALL_DIR "$PROGRAMFILES64\WireSentinel"
!define DATA_DIR "$COMMONAPPDATA\WireSentinel"
!define API_PORT "8170"

; Override via environment when built by scripts/build-installer.ps1
!ifndef WIRESENTINEL_VERSION
  !define WIRESENTINEL_VERSION "${PRODUCT_VERSION}"
!endif
!ifndef WIRESENTINEL_ARCH
  !define WIRESENTINEL_ARCH "x64"
!endif

Name "${PRODUCT_NAME} ${WIRESENTINEL_VERSION}"
OutFile "..\..\dist\WireSentinel-${WIRESENTINEL_VERSION}-${WIRESENTINEL_ARCH}-setup.exe"
InstallDir "${INSTALL_DIR}"
RequestExecutionLevel admin
ShowInstDetails nevershow
ShowUnInstDetails nevershow

Var /GLOBAL InstallKernelDrivers

Function .onInit
  StrCpy $InstallKernelDrivers "1"
  !insertmacro SelectSection ${SecKernelDrivers}
FunctionEnd

Section "WireSentinel application" SecMain
  SetOutPath "$INSTDIR"

  File "..\..\installer\staging\bin\wire-sentinel-service.exe"
  File "..\..\installer\staging\bin\wire-sentinel.exe"
  File "..\..\resources\tunnel.dll"
  File "..\..\resources\wireguard.dll"
  !insertmacro InstallThirdPartyFiles

  SetOutPath "$INSTDIR\scripts"
  File "..\..\scripts\install-kernel-drivers.ps1"
  File "..\..\scripts\uninstall-kernel-drivers.ps1"

  ; ProgramData directories (logs, tunnels, transports)
  CreateDirectory "${DATA_DIR}"
  CreateDirectory "${DATA_DIR}\logs"
  CreateDirectory "${DATA_DIR}\tunnels"
  CreateDirectory "${DATA_DIR}\transports"

  ; Register Windows Service (manual start — demand)
  nsExec::ExecToLog 'sc.exe create WireSentinel binPath= "\"$INSTDIR\wire-sentinel-service.exe\"" start= demand'
  nsExec::ExecToLog 'sc.exe description WireSentinel "WireSentinel network security service"'
  nsExec::ExecToLog 'sc.exe failure WireSentinel reset= 86400 actions= restart/60000/restart/60000/restart/60000'
  nsExec::ExecToLog 'sc.exe config WireSentinel depend= Tcpip/Dnscache'

  ; Loopback API firewall rule (127.0.0.1:${API_PORT})
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="WireSentinel API (loopback)" dir=in action=allow protocol=TCP localport=${API_PORT} remoteip=127.0.0.1 profile=any'

  ; Start Menu shortcut
  CreateDirectory "$SMPROGRAMS\WireSentinel"
  CreateShortcut "$SMPROGRAMS\WireSentinel\WireSentinel.lnk" "$INSTDIR\wire-sentinel.exe"
  CreateShortcut "$DESKTOP\WireSentinel.lnk" "$INSTDIR\wire-sentinel.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "!Kernel drivers (Guardian + NDIS)" SecKernelDrivers
  SetOutPath "$INSTDIR\drivers\guardian"
  File "..\..\installer\staging\drivers\current\guardian\Guardian.sys"
  File "..\..\installer\staging\drivers\current\guardian\guardian.inf"
  File /nonfatal "..\..\installer\staging\drivers\current\guardian\Guardian.cat"

  SetOutPath "$INSTDIR\drivers\ndis"
  File "..\..\installer\staging\drivers\current\ndis\guardian_lwf.sys"
  File "..\..\installer\staging\drivers\current\ndis\guardian_lwf.inf"
  File /nonfatal "..\..\installer\staging\drivers\current\ndis\guardian_lwf.cat"

  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-kernel-drivers.ps1" -DriverRoot "$INSTDIR\drivers" -Mode Install'
  WriteRegDWORD HKLM "SOFTWARE\WireSentinel\Installer" "KernelDrivers" 1
SectionEnd

Function .onSelChange
  ${If} ${SectionIsSelected} ${SecKernelDrivers}
    StrCpy $InstallKernelDrivers "1"
  ${Else}
    StrCpy $InstallKernelDrivers "0"
    WriteRegDWORD HKLM "SOFTWARE\WireSentinel\Installer" "KernelDrivers" 0
  ${EndIf}
FunctionEnd

Function un.onInit
  ReadRegDWORD $InstallKernelDrivers HKLM "SOFTWARE\WireSentinel\Installer" "KernelDrivers"
  ${If} $InstallKernelDrivers == ""
    StrCpy $InstallKernelDrivers "0"
  ${EndIf}
FunctionEnd

Section "Uninstall"
  ${If} $InstallKernelDrivers == "1"
    nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\scripts\install-kernel-drivers.ps1" -DriverRoot "$INSTDIR\drivers" -Mode Uninstall'
  ${EndIf}

  nsExec::ExecToLog 'sc.exe stop WireSentinel'
  nsExec::ExecToLog 'sc.exe delete WireSentinel'
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="WireSentinel API (loopback)"'

  Delete "$INSTDIR\wire-sentinel-service.exe"
  Delete "$INSTDIR\wire-sentinel.exe"
  Delete "$INSTDIR\tunnel.dll"
  Delete "$INSTDIR\wireguard.dll"
  !insertmacro UninstallThirdPartyFiles
  Delete "$INSTDIR\scripts\install-kernel-drivers.ps1"
  Delete "$INSTDIR\scripts\uninstall-kernel-drivers.ps1"
  RMDir "$INSTDIR\scripts"
  RMDir /r "$INSTDIR\drivers"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$DESKTOP\WireSentinel.lnk"
  RMDir /r "$SMPROGRAMS\WireSentinel"
  DeleteRegKey HKLM "SOFTWARE\WireSentinel\Installer"
  RMDir "$INSTDIR"
SectionEnd
