# StrataFetch v0.1.0-beta.4 — Stability update

**[Download the Windows setup installer](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.4/stratafetch-windows-x64-setup.exe)**

This release fixes issues found during a code audit and reproduced with local regression tests. It retains the beta.3 Windows startup fix and includes matching Windows and Linux builds.

## Fixes

- Retry an interrupted range-probe body instead of immediately failing the download. The existing retry budget still applies; inconsistent range headers remain rejected.
- Keep progress and durable checkpoints moving at low individual and global speed limits. Data is paced in small pieces before writing, so pausing no longer commits a large unpaced chunk.
- Wait for a briefly busy Windows local-service pipe. Only connection attempts are retried, so an add/remove command is never automatically replayed.
- Discard pending desktop commands on exit instead of draining the queue and starting new work during shutdown. An operation already in progress finishes before the engine pauses its transfers.
- Reset average-speed measurements after restarting a transfer and release per-download UI tracking when history is removed.
- Fix Windows package verification in CRLF source checkouts and record the actual Windows build target.

## Install and update

- **stratafetch-windows-x64-setup.exe**: recommended Windows download; per-user setup wizard, optional Start Menu shortcuts and Windows Installed Apps uninstall entry. Close StrataFetch desktop/service before upgrading. Profiles and completed downloads are preserved.
- **stratafetch-linux-x64.tar.gz**: desktop, CLI, native browser host and optional application-menu integration. Built on Ubuntu 24.04; requires GLIBC_2.39 or newer, X11/Wayland and compatible Vulkan support.
- **stratafetch-desktop.exe**, **stratafetch-windows-x64.zip** and **SHA256SUMS**: desktop updater asset, full Windows bundle and checksums for all downloads.

Tools → Check for updates supports verified desktop replacement and restart on Windows x64. Use the setup installer to upgrade the CLI and browser host together. Linux update checks open the release download page. Beta.2 users whose desktop cannot start must install the setup manually.

## Verification

**88 Rust tests pass on Linux and 82 pass natively on Windows**, including platform-specific regressions. Strict Clippy, formatting and all 18 browser-script tests pass. Actual Windows/Linux CLI and native-host downloads, desktop rendering, an isolated Windows beta.3 → beta.4 updater restart and installer install/reinstall/uninstall pass. ZIP contents, embedded Windows manifests and SHA-256 hashes are verified. [Detailed audit and evidence](https://github.com/Sub2Nex/stratafetch/blob/main/docs/STABILITY-BETA4.md). Each archive records its application build revision in BUILD.txt.

All builds use local WSL environments without CI or GitHub Actions. Windows binaries are unsigned. This beta does not claim exhaustive reliability, code-signing reputation, macOS/ARM support or complete installed-browser/accessibility testing.
