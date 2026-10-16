# StrataFetch v0.1.0-beta.2 — desktop updates (source only)

This release contains the updater source changes. **There are no beta.2 executable downloads.** Windows application control prevented the Rust build from completing (OS error 4551), so compilation, regression tests and an actual upgrade cycle remain unverified. This is a development beta, not a validated stable release.

## Added

- Tools → Check for updates, with automatic checks on startup and every 24 hours while open.
- A saved setting to disable automatic checks.
- Windows x64 desktop update downloads with SHA-256 verification and an explicit Restart and install action.
- A helper that waits for normal shutdown, retains the previous executable and restores it if replacement or launch fails.
- Version/channel selection: stable builds ignore previews; preview builds can receive newer previews and stable releases.
- Update packaging instructions, privacy documentation and five regression tests covering version selection, checksum parsing, asset URL restrictions and replacement/recovery.

## Downloads and limitations

GitHub's source ZIP and tarball contain the new code. They are not runnable application packages. Existing [beta.1 application downloads](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.1) remain available separately and do not contain the updater. No previous-version binary is attached or relabeled as beta.2.

The in-app installer targets Windows x64 desktop executables. CLI, browser integration and other platforms require the full package or release downloads. Existing installations will need one manual upgrade to obtain an updater-enabled binary when one is available. Future compatible releases must include `stratafetch-desktop.exe` and `SHA256SUMS` and preserve the update helper protocol.

Formatting and whitespace checks passed. Build, Clippy, tests, native UI and update/recovery checks must pass on an allowed build machine before binary distribution. Checksums detect corruption; they are not independent publisher signatures. No GitHub Actions workflows or hosted build resources were enabled.
