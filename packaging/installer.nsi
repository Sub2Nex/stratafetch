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
Caption "$(SetupCaption)"
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
      MessageBox MB_OK "$(CloseFirst)" /SD IDOK
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
!define MUI_FINISHPAGE_RUN_TEXT "$(RunApp)"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchApp
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Swedish"
LangString SetupCaption ${LANG_ENGLISH} "StrataFetch ${APP_VERSION} Setup"
LangString SetupCaption ${LANG_SWEDISH} "Installation av StrataFetch ${APP_VERSION}"
LangString RunApp ${LANG_ENGLISH} "Launch StrataFetch"
LangString RunApp ${LANG_SWEDISH} "Starta StrataFetch"
LangString CoreComponent ${LANG_ENGLISH} "StrataFetch desktop and command line"
LangString CoreComponent ${LANG_SWEDISH} "StrataFetch-app och kommandorad"
LangString ShortcutsComponent ${LANG_ENGLISH} "Start Menu shortcuts"
LangString ShortcutsComponent ${LANG_SWEDISH} "Genvägar på Start-menyn"
LangString BrowserComponent ${LANG_ENGLISH} "Local browser integration"
LangString BrowserComponent ${LANG_SWEDISH} "Lokal webbläsarintegrering"
LangString CloseFirst ${LANG_ENGLISH} "Close StrataFetch and its background service, then try again. No application files have been changed. Download history is preserved."
LangString CloseFirst ${LANG_SWEDISH} "Avsluta StrataFetch och bakgrundstjänsten och försök igen. Inga programfiler har ändrats. Hämtningshistoriken behålls."
LangString Windows64 ${LANG_ENGLISH} "StrataFetch requires 64-bit Windows."
LangString Windows64 ${LANG_SWEDISH} "StrataFetch kräver 64-bitars Windows."
LangString CopyFailed ${LANG_ENGLISH} "Installation could not copy all files. Close StrataFetch and its background service, then run the installer again. Download history is preserved."
LangString CopyFailed ${LANG_SWEDISH} "Installationen kunde inte kopiera alla filer. Avsluta StrataFetch och bakgrundstjänsten och kör installationen igen. Hämtningshistoriken behålls."
LangString BrowserSetupNeeded ${LANG_ENGLISH} "StrataFetch is installed. Local browser setup needs attention: open Settings > Browser integration. Existing registration was preserved."
LangString BrowserSetupNeeded ${LANG_SWEDISH} "StrataFetch är installerat. Webbläsarkonfigurationen behöver slutföras: öppna Inställningar > Webbläsarintegrering. Den befintliga registreringen behölls."
LangString BrowserRemovalFailed ${LANG_ENGLISH} "Browser registration could not be removed automatically. See docs/BROWSER.md. Another installation's registration is preserved."
LangString BrowserRemovalFailed ${LANG_SWEDISH} "Webbläsarregistreringen kunde inte tas bort automatiskt. Se docs/BROWSER.md. Registrering som tillhör en annan installation behålls."
LangString FilesInUse ${LANG_ENGLISH} "Some StrataFetch executable files are still in use. Close the desktop and service, then run this uninstaller again. Download history and the Windows uninstall entry are preserved."
LangString FilesInUse ${LANG_SWEDISH} "Några programfiler används fortfarande. Avsluta appen och tjänsten och kör avinstallationen igen. Hämtningshistoriken och avinstallationsposten behålls."
Function .onInit
  SetRegView 64
  ReadRegStr $0 HKCU "Software\StrataFetch" "Language"
  ${If} $0 == "sv"
    StrCpy $LANGUAGE ${LANG_SWEDISH}
  ${ElseIf} $0 == "en"
    StrCpy $LANGUAGE ${LANG_ENGLISH}
  ${EndIf}
  ${IfNot} ${RunningX64}
    MessageBox MB_OK "$(Windows64)"
    Abort
  ${EndIf}
FunctionEnd
Function un.onInit
  SetRegView 64
FunctionEnd
Section "$(CoreComponent)" Core
  SectionIn RO
  Call CheckApplicationClosed
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ClearErrors
  File /r "${PACKAGE_DIR}\*.*"
  ${If} ${Errors}
    MessageBox MB_OK "$(CopyFailed)" /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "StrataFetch"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" '$\"$INSTDIR\stratafetch.ico$\",0'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/Sub2Nex/stratafetch"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd
Section "$(ShortcutsComponent)" Shortcuts
  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\${START_MENU_DIR}"
  CreateShortcut "$SMPROGRAMS\${START_MENU_DIR}\StrataFetch.lnk" "$INSTDIR\stratafetch-desktop.exe" "" "$INSTDIR\stratafetch.ico" 0
  CreateShortcut "$SMPROGRAMS\${START_MENU_DIR}\Uninstall.lnk" "$INSTDIR\Uninstall.exe" "" "$INSTDIR\stratafetch.ico" 0
SectionEnd
Function .onInstSuccess
  ; Refresh Installed apps and shortcuts after all selected sections finish.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
FunctionEnd
Section "$(BrowserComponent)" Browsers
  !ifdef TEST_BUILD
    DetailPrint "Isolated installer test: browser registration is verified separately."
  !else
    nsExec::ExecToLog '"$INSTDIR\stratafetch-browser-host.exe" --register-local'
    Pop $0
    ${If} $0 != 0
      MessageBox MB_OK "$(BrowserSetupNeeded)" /SD IDOK
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
    MessageBox MB_OK "$(BrowserRemovalFailed)" /SD IDOK
  ${EndIf}
  !endif
  ; Only remove the product files we installed. Preserve other files and all user state.
  ClearErrors
  Delete "$INSTDIR\stratafetch.exe"
  Delete "$INSTDIR\stratafetch-desktop.exe"
  Delete "$INSTDIR\stratafetch-browser-host.exe"
  ${If} ${Errors}
    MessageBox MB_OK "$(FilesInUse)" /SD IDOK
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
