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

Regression fixtures cover disconnected probes with retry exhaustion, low-limit progress/pause/resume with exact output bytes, pending-command shutdown, restarted speed measurements and busy Windows pipe admission. The full Rust suite, strict Clippy, browser scripts, native executable smoke checks and isolated installer roundtrip are run locally before publication. Each archive records its source revision in BUILD.txt.

All builds use local WSL environments without CI or GitHub Actions. Windows binaries are unsigned. This beta does not claim exhaustive reliability, code-signing reputation, macOS/ARM support or complete installed-browser/accessibility testing.
