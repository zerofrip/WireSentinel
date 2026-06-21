; WireSentinel NSIS installer
; Build: makensis installer/nsis/installer.nsi
; Silent: WireSentinel-0.1.0-setup.exe /S

!include "LogicLib.nsh"

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

Function .onInit
  ; NSIS handles /S automatically; avoid interactive prompts during silent install.
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"

  File "..\..\target\release\wire-sentinel-service.exe"
  File "..\..\ui\src-tauri\target\release\wire-sentinel.exe"
  File "..\..\resources\tunnel.dll"
  File "..\..\resources\wireguard.dll"

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

Section "Uninstall"
  nsExec::ExecToLog 'sc.exe stop WireSentinel'
  nsExec::ExecToLog 'sc.exe delete WireSentinel'
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="WireSentinel API (loopback)"'

  Delete "$INSTDIR\wire-sentinel-service.exe"
  Delete "$INSTDIR\wire-sentinel.exe"
  Delete "$INSTDIR\tunnel.dll"
  Delete "$INSTDIR\wireguard.dll"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$DESKTOP\WireSentinel.lnk"
  RMDir /r "$SMPROGRAMS\WireSentinel"
  RMDir "$INSTDIR"
SectionEnd
