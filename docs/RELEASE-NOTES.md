# StrataFetch v0.1.0-beta.2 — desktop updates

The updater is now available in unsigned Windows x64 application downloads, built locally in Ubuntu WSL2 from the exact published beta.2 source tag. This is a development beta; native Windows execution and the full in-app upgrade/restart cycle remain unverified.

## Downloads

- **stratafetch-desktop.exe** — standalone Windows x64 desktop with the updater. Portable, not an installer.
- **stratafetch-windows-x64.zip** — matching Windows desktop, CLI, browser host, extensions, documentation and dependency notices.
- **SHA256SUMS** — SHA-256 hashes for the EXE and ZIP.

Extract the ZIP into a writable permanent folder, or replace the old standalone desktop executable after closing StrataFetch. Existing users need one manual upgrade to obtain the updater; subsequent compatible desktop releases can be downloaded from within the app. Interrupted transfers reopen paused. Use the full ZIP to upgrade CLI/browser integration together. Linux downloads remain available under [beta.1](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.1); no beta.2 Linux/macOS/ARM64 package or installer is included.

## Added

- **Tools → Check for updates**, with automatic checks on startup and every 24 hours while open.
- A saved setting to disable automatic checks.
- Windows x64 desktop update downloads with SHA-256 verification and an explicit **Restart and install** action.
- A helper that waits for normal shutdown, retains the previous executable and restores it if replacement or launch fails.
- Version/channel selection: stable builds ignore previews; preview builds can receive newer previews and stable releases.
- Update packaging instructions, privacy documentation and regression tests.

## Verification and limits

- Built all three Windows binaries with Rust 1.99.1 for `x86_64-pc-windows-gnu`, using the source tag `v0.1.0-beta.2`, commit `3dcd1b4211ec84487dac8029e9791c554a6efd19`.
- Formatting, strict Clippy and all **84 Rust tests** pass on Linux, including the five updater/version/checksum/recovery tests.
- All **19 browser/CLI/native-host checks** pass, including a real isolated ranged download.
- The native Linux debug desktop renders under WSLg with the automatic-update setting visible and fresh isolated state. This does not establish Windows runtime or physical-device acceptance.
- Package checks verify x64 PE format, GUI/console subsystems, runtime imports, ZIP integrity and matching executable hashes. Publication documentation in the ZIP is updated to describe the subsequently added binaries; executable source remains the published tag.

Windows application control blocked the native Rust build scripts. A separate local Ubuntu environment performed the cross-build; no security policy was disabled. Windows binaries have not been executed on Windows, and signing, DPI/accessibility, installed-browser integration and the full in-app upgrade/restart cycle remain pending. The helper retains a `.stratafetch-update-*` recovery directory. SHA-256 detects corruption but is not an independent publisher signature. Follow applicable application-control policies when running unsigned software.

This release was initially published with source archives only. The newly attached assets contain the actual beta.2 build, not relabeled beta.1 binaries. No CI, GitHub Actions or hosted build resources were used.
