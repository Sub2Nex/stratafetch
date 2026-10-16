Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "Sections.nsh"
!ifndef PACKAGE_DIR
!define PACKAGE_DIR "..\dist\stratafetch-windows-x64"
!endif
!ifndef APP_VERSION
!error "Build with build-installer.py so APP_VERSION is set from Cargo.toml"
!endif
!ifndef PRODUCT_VERSION
!error "PRODUCT_VERSION must be set by the installer build script"
!endif
!ifndef OUTPUT_FILE
!define OUTPUT_FILE "..\dist\stratafetch-windows-x64-setup.exe"
!endif
!ifndef UNINSTALL_KEY
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch"
!endif
!ifndef START_MENU_DIR
!define START_MENU_DIR "StrataFetch"
!endif
!ifndef INSTALLER_FILES
!define INSTALLER_FILES "installer-files.nsh"
!endif
Name "StrataFetch ${APP_VERSION}"
Caption "StrataFetch ${APP_VERSION} Setup"
BrandingText "StrataFetch"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\StrataFetch"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
; Verify every executable before changing any payload. A running desktop must
; not leave the CLI/bridge deleted or updated while its own EXE stays locked.
!macro CheckExecutableClosed Filename
  ${If} ${FileExists} "$INSTDIR\${Filename}"
    System::Call 'kernel32::CreateFileW(w "$INSTDIR\${Filename}", i 0x40000000, i 0, p 0, i 3, i 0, p 0) p.r0'
    ${If} $0 == -1
      MessageBox MB_OK "Close StrataFetch and its background service, then try again. No application files have been changed. Download history is preserved." /SD IDOK
      SetErrorLevel 1
      Abort
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p r0)'
  ${EndIf}
!macroend
Function CheckApplicationClosed
  !insertmacro CheckExecutableClosed "stratafetch.exe"
  !insertmacro CheckExecutableClosed "stratafetch-desktop.exe"
  !insertmacro CheckExecutableClosed "stratafetch-browser-host.exe"
FunctionEnd
Function un.CheckApplicationClosed
  !insertmacro CheckExecutableClosed "stratafetch.exe"
  !insertmacro CheckExecutableClosed "stratafetch-desktop.exe"
  !insertmacro CheckExecutableClosed "stratafetch-browser-host.exe"
FunctionEnd

VIProductVersion "${PRODUCT_VERSION}"
VIAddVersionKey "ProductName" "StrataFetch"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "StrataFetch Windows setup installer"
VIAddVersionKey "LegalCopyright" "StrataFetch contributors"
!define MUI_ICON "..\assets\stratafetch.ico"
!define MUI_UNICON "..\assets\stratafetch.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE-MIT"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\stratafetch-desktop.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch StrataFetch"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchApp
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Function .onInit
  SetRegView 64
  ${IfNot} ${RunningX64}
    MessageBox MB_OK "StrataFetch requires 64-bit Windows."
    Abort
  ${EndIf}
FunctionEnd
Function un.onInit
  SetRegView 64
FunctionEnd
Section "StrataFetch desktop and command line" Core
  SectionIn RO
  Call CheckApplicationClosed
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
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "StrataFetch"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\stratafetch-desktop.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/Sub2Nex/stratafetch"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd
Section "Start Menu shortcuts" Shortcuts
  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\${START_MENU_DIR}"
  CreateShortcut "$SMPROGRAMS\${START_MENU_DIR}\StrataFetch.lnk" "$INSTDIR\stratafetch-desktop.exe"
  CreateShortcut "$SMPROGRAMS\${START_MENU_DIR}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd
Section "Local browser integration" Browsers
  !ifdef TEST_BUILD
    DetailPrint "Isolated installer test: browser registration is verified separately."
  !else
    nsExec::ExecToLog '"$INSTDIR\stratafetch-browser-host.exe" --register-local'
    Pop $0
    ${If} $0 != 0
      MessageBox MB_OK "StrataFetch is installed. Local browser setup needs attention: open Settings > Browser integration. Existing registration was preserved." /SD IDOK
    ${EndIf}
  !endif
SectionEnd
Function LaunchApp
  SectionGetFlags ${Browsers} $0
  IntOp $0 $0 & ${SF_SELECTED}
  ${If} $0 != 0
    Exec '"$INSTDIR\stratafetch-desktop.exe" --browser-setup'
  ${Else}
    Exec '"$INSTDIR\stratafetch-desktop.exe"'
  ${EndIf}
FunctionEnd
Section "Uninstall"
  SetShellVarContext current
  Call un.CheckApplicationClosed
  !ifndef TEST_BUILD
  nsExec::ExecToLog '"$INSTDIR\stratafetch-browser-host.exe" --unregister-local'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_OK "Browser registration could not be removed automatically. See docs/BROWSER.md. Another installation's registration is preserved." /SD IDOK
  ${EndIf}
  !endif
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
  !include "${INSTALLER_FILES}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${START_MENU_DIR}\StrataFetch.lnk"
  Delete "$SMPROGRAMS\${START_MENU_DIR}\Uninstall.lnk"
  RMDir "$SMPROGRAMS\${START_MENU_DIR}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
