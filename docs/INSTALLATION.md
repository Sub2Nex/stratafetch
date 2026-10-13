# Windows packages

StrataFetch is an unsigned development preview. Public source: https://github.com/Sub2Nex/stratafetch. The unsigned Windows alpha remains experimental; see RELEASE-NOTES.md for its limits.

## Portable ZIP

Extract `stratafetch-windows-x64.zip` to a permanent folder and run `stratafetch-desktop.exe`. The CLI is `stratafetch.exe`. No Rust installation is needed for binaries. Verify SHA-256 against `SHA256SUMS` using `Get-FileHash` before running downloaded packages. Treat unsigned publisher/application-control warnings accurately; the project does not recommend disabling security protection.

## Per-user installer

`stratafetch-windows-x64-setup.exe` uses NSIS 3.13 and defaults to `%LOCALAPPDATA%\Programs\StrataFetch`. It installs all three executables, documentation, license notices and browser extension sources. Start Menu shortcuts are optional. Administrator access is not requested. Close the application before installing upgrades or uninstalling.

Uninstall through Windows Installed Apps or `Uninstall.exe`. It removes the known installed files, its shortcuts and its HKCU application entry. Other files placed in the installation folder are preserved. Downloads and SQLite state under `%LOCALAPPDATA%\StrataFetch\StrataFetch\data` are preserved. Uninstallation attempts to remove matching native-host registration; another installation's registration is preserved. If PowerShell policy blocks cleanup, use the documented manual unregistration command in BROWSER.md. Uninstall does not remove browser extensions; remove those through the browser.

Browser registration is deliberately separate: see BROWSER.md. No browser interception, autostart, default-handler registration, or file association is silently enabled.

## Build packages

```powershell
cargo build --release --workspace --locked
cargo run --release -p strata-app --example branding --locked
./packaging/portable.ps1
./packaging/build-installer.ps1 -Compiler 'C:\path\to\NSIS\Bin\makensis.exe'
```

Get NSIS from its official distribution: https://nsis.sourceforge.io/Download. The build generates a file-by-file uninstall list; it does not recursively delete the installation directory. Both outputs are recorded in `dist/SHA256SUMS`. BUILD.txt records the committed source revision. Build from a clean committed checkout for reproducibility.

The isolated installer smoke test installed into a workspace folder, compared the installed desktop executable, ran silent uninstallation, and confirmed removal of application files/registration while preserving an unrelated sentinel file. Default-path upgrade behavior and interactive installer page navigation still require further review.

On machines with little free memory, pass -j 1 to Cargo commands. Final local parallel compilation encountered Windows error 1455 (paging-memory exhaustion); a single build job avoided that resource failure.

Run ./packaging/verify-package.ps1 to compare archive/build hashes and source metadata, then ./packaging/test-installer.ps1 for an isolated install/uninstall roundtrip. These scripts run locally. The installer test stops if an existing user installation or Start Menu folder would be affected.

Final alpha installer execution was blocked by Windows application control on the build machine. It is compiled and hash-verified, but its final installation roundtrip is unverified. The earlier preview roundtrip passed. The portable release binaries passed native/CLI execution checks. Do not disable protection to run the installer.

## Alpha.2 portable-only release

The desktop redesign prerelease includes the portable ZIP and SHA256SUMS. No new installer roundtrip is claimed. The NSIS sections above describe the historical alpha.1 package and optional installer source. For alpha.2, build from the committed source, run ./packaging/portable.ps1, then ./packaging/verify-package.ps1 -PortableOnly. Close older desktop/service instances before running the new version. Do not mix old engine processes with the new desktop's additive paused-add commands.
