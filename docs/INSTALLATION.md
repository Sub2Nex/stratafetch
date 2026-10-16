# Windows packages

## Current beta.4 setup installer

Download `stratafetch-windows-x64-setup.exe` from the beta.4 release and open it to launch the setup wizard. Choose a destination and optional Start Menu shortcuts, then use the finish-page checkbox to launch StrataFetch. The default location is `%LOCALAPPDATA%\Programs\StrataFetch`, and installation is per-user without administrator access. Windows Installed Apps and the Start Menu uninstall shortcut can remove it. Profiles, completed downloads and unrelated files are preserved. Close the desktop/service before upgrades or removal.

The versioned NSIS build takes its application/version metadata from Cargo.toml and installs the same verified beta.4 desktop/CLI/browser-host binaries. `python packaging/build-installer.py --package PATH --output PATH` can build locally with NSIS on Linux or Windows; `build-installer.ps1` is the Windows wrapper. Python 3.11+ is required for installer builds. The earlier installer sections below describe historical alpha behavior and evidence.

Installer tests use a separate registry/shortcut namespace and a workspace destination, preserving the existing user installation. They verify all three installed binary hashes, application version, reinstall, uninstall and preservation of an unrelated sentinel file. Beta.4 includes both application and installer sources in the release tag.

## Portable ZIP

Windows packaging also produces a standalone `stratafetch-desktop.exe` for direct download. When offered as a release asset, save it to a permanent folder and run it to launch the desktop. It is portable, not an installer. Use the ZIP below for the CLI, native browser host, extensions and documentation. Verify the executable against the release's `SHA256SUMS` (or `SHA256SUMS-desktop` for an executable added to an existing release).

Extract `stratafetch-windows-x64.zip` to a permanent folder and run `stratafetch-desktop.exe`. The CLI is `stratafetch.exe`. No Rust installation is needed for binaries. Verify SHA-256 against `SHA256SUMS` using `Get-FileHash` before running downloaded packages. Treat unsigned publisher/application-control warnings accurately; the project does not recommend disabling security protection.

## Per-user installer

`stratafetch-windows-x64-setup.exe` uses NSIS 3.13 and defaults to `%LOCALAPPDATA%\Programs\StrataFetch`. It installs all three executables, documentation, license notices and browser extension sources. Start Menu shortcuts are optional. Administrator access is not requested. Close the application before installing upgrades or uninstalling.

Uninstall through Windows Installed Apps or `Uninstall.exe`. It removes the known installed files, its shortcuts and its HKCU application entry. Other files placed in the installation folder are preserved. Downloads and SQLite state under `%LOCALAPPDATA%\StrataFetch\StrataFetch\data` are preserved. Uninstallation attempts to remove matching native-host registration; another installation's registration is preserved. If PowerShell policy blocks cleanup, use the documented manual unregistration command in BROWSER.md. Uninstall does not remove browser extensions; remove those through the browser.

Browser registration is deliberately separate: see BROWSER.md. No browser interception, autostart, default-handler registration, or file association is silently enabled.

## Build packages

Windows packaging requires Python 3 to run `scripts/verify-windows-manifest.py`. The check rejects executables without the Common Controls v6 application manifest before a ZIP is created. MSVC builds embed the manifest with the native linker; GNU builds use the target's `windres` resource compiler.

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

## v0.1.0-alpha.3 portable release

Download the ZIP and SHA256SUMS from https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-alpha.3. Close the existing desktop and service, back up user state, and replace all three executables together. New IPC clients require the matching service. Re-register the native host if its permanent executable path changes. Default download history lives outside the installation folder; schema 2 and existing IDs/checkpoints are preserved.

This release distributes no installer. The compiled installer still lacks runtime upgrade/uninstall acceptance. Unsigned executables can be rejected by Windows application control; do not change security settings to force them to run.

## v0.1.0-beta.1 Windows and Linux release

Download the standalone Windows desktop `.exe`, full Windows ZIP, Linux x64 archive and shared `SHA256SUMS` from https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.1. Check each downloaded file against that checksum manifest. The Windows binaries are unsigned GNU cross-builds and have not been executed on Windows. The Linux archive was verified on Debian 13 x64 and the desktop requires GLIBC_2.39; older distributions need a native rebuild. No installer or macOS binary is included.

Close existing desktop/service processes, back up state and upgrade all three programs together when using the full package. Keep the unpacked installation in a permanent directory before registering browser hosts. On Linux, run `./stratafetch install-desktop` to add the per-user application-menu entry. See [RELEASE-NOTES.md](RELEASE-NOTES.md) for validation and remaining platform checks.

## Linux and macOS development packages

Run `cargo build --release --workspace --bins --locked`, then `bash packaging/portable.sh` on the target OS/architecture. Linux gets an architecture-specific archive and per-user menu integration through `stratafetch install-desktop`. macOS gets an architecture-specific archive containing `StrataFetch.app`, its icon/manifest and all three executables. Each archive has a separate platform-specific checksum file, preserving Windows checksums. See [PLATFORMS.md](PLATFORMS.md) for prerequisites, tested capabilities and outstanding native Mac validation. These development changes are not part of the historical alpha.3 assets.
