Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!ifndef PACKAGE_DIR
!define PACKAGE_DIR "..\dist\stratafetch-windows-x64"
!endif
Name "StrataFetch (development preview)"
OutFile "..\dist\stratafetch-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\StrataFetch"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "0.1.0.0"
VIAddVersionKey "ProductName" "StrataFetch"
VIAddVersionKey "ProductVersion" "0.1.0-alpha.3"
VIAddVersionKey "FileVersion" "0.1.0-alpha.3"
VIAddVersionKey "FileDescription" "StrataFetch unsigned Windows alpha installer"
VIAddVersionKey "LegalCopyright" "StrataFetch contributors"
!define MUI_ICON "..\assets\stratafetch.ico"
!define MUI_UNICON "..\assets\stratafetch.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE-MIT"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK "StrataFetch requires 64-bit Windows."
    Abort
  ${EndIf}
FunctionEnd
Section "StrataFetch desktop and command line" Core
  SectionIn RO
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ClearErrors
  File /r "${PACKAGE_DIR}\*.*"
  ${If} ${Errors}
    MessageBox MB_OK "Installation could not copy all files. Close StrataFetch and its background service, then run the installer again. Download history is preserved." /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "DisplayName" "StrataFetch (development preview)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "DisplayVersion" "0.1.0-alpha.3"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "URLInfoAbout" "https://github.com/Sub2Nex/stratafetch"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch" "NoRepair" 1
SectionEnd
Section "Start Menu shortcuts" Shortcuts
  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\StrataFetch"
  CreateShortcut "$SMPROGRAMS\StrataFetch\StrataFetch.lnk" "$INSTDIR\stratafetch-desktop.exe"
  CreateShortcut "$SMPROGRAMS\StrataFetch\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd
Section "Uninstall"
  SetShellVarContext current
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -File "$INSTDIR\packaging\register-browser-host.ps1" -Uninstall -BinaryPath "$INSTDIR\stratafetch-browser-host.exe"'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_OK "Browser registration could not be removed automatically. See docs/BROWSER.md. Another installation's registration is preserved." /SD IDOK
  ${EndIf}
  ; Only remove the product files we installed. Preserve other files and all user state.
  ClearErrors
  Delete "$INSTDIR\stratafetch.exe"
  Delete "$INSTDIR\stratafetch-desktop.exe"
  Delete "$INSTDIR\stratafetch-browser-host.exe"
  ${If} ${Errors}
    MessageBox MB_OK "Some StrataFetch executable files are still in use. Close the desktop and service, then run this uninstaller again. Download history and the Windows uninstall entry are preserved." /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
  Delete "$INSTDIR\BUILD.txt"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE-MIT"
  Delete "$INSTDIR\LICENSE-APACHE"
  Delete "$INSTDIR\THIRD-PARTY-LICENSES.txt"
  Delete "$INSTDIR\PRIVACY.md"
  Delete "$INSTDIR\SECURITY.md"
  Delete "$INSTDIR\ARCHITECTURE.md"
  Delete "$INSTDIR\CONTRIBUTING.md"
  Delete "$INSTDIR\CODE_OF_CONDUCT.md"
  Delete "$INSTDIR\ROADMAP.md"
  Delete "$INSTDIR\CHANGELOG.md"
  ; Explicit directory contents are generated below by build-installer.ps1.
  !include "installer-files.nsh"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\StrataFetch\StrataFetch.lnk"
  Delete "$SMPROGRAMS\StrataFetch\Uninstall.lnk"
  RMDir "$SMPROGRAMS\StrataFetch"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch"
SectionEnd
