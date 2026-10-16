# StrataFetch v0.1.0-beta.3 — Windows startup fix and Linux x64

Fixes the beta.2 Windows launch failure: “TaskDialogIndirect could not be located.” Every Windows executable now embeds the Common Controls v6 activation manifest required by the native dialogs. This release also supplies a matching Linux x64 package.

## Downloads

- **stratafetch-desktop.exe** — fixed standalone Windows x64 desktop.
- **stratafetch-windows-x64.zip** — matching Windows desktop, CLI, browser host, extensions and documentation.
- **stratafetch-linux-x64.tar.gz** — native Linux x64 desktop, CLI, browser host and desktop-menu integration. Built locally on Ubuntu 24.04; requires GLIBC_2.39 or newer, X11/Wayland and a compatible Vulkan driver.
- **SHA256SUMS** — hashes for all three application downloads.

Close older StrataFetch processes and replace the old executable or extract the full package into a permanent directory. **The broken beta.2 Windows desktop requires a manual upgrade to beta.3.** Preserve profiles and completed downloads. On Linux, run `./stratafetch-desktop`; optional `./stratafetch install-desktop` adds the per-user application-menu entry.

## Fix and update behavior

- Embed and validate the Windows process manifest for GNU and MSVC builds. Packaging now rejects executables missing Common Controls v6, reproducing and preventing the beta.2 packaging defect.
- Retain automatic and manual update checks. Automatic checks can be disabled in Settings → General.
- Windows x64 supports download, SHA-256 verification and explicit Restart and install. Linux checks for releases and opens their download page; it does not replace the installed Linux package automatically.
- The CLI, browser host and extensions are updated using the full platform package.

## Verification

- Windows binaries built locally for `x86_64-pc-windows-gnu` from code commit `c33d784ffafe10c9687137d9c3baa85f076309aa`.
- The manifest check rejects beta.2 and accepts all three beta.3 Windows executables. The native Windows SDK extracts and validates the embedded desktop manifest.
- **Native Windows CLI and desktop startup pass.** The real desktop renders settings with fresh isolated state and exits cleanly.
- **Native Windows updater-helper replacement and restart pass.** An isolated test retains the exact previous executable, installs the exact new executable, relaunches it and captures the working desktop. This does not validate every GUI download/install interaction or application-control policy.
- All **84 Rust tests** and strict Clippy pass on Linux; the actual Linux release CLI/native-host completes an isolated ranged download with exact-byte/SHA-256 verification.
- Package verification checks executable formats, embedded manifests, archive contents, executable/archive byte consistency and SHA-256 hashes.

All builds run locally in WSL; no CI or GitHub Actions are used. Windows binaries remain unsigned. Browser safety scans can still run, and a successful startup test is not a security-reputation or signing guarantee. No Windows/browser security protections were disabled. Installed-browser integration, physical-device DPI/accessibility transitions and macOS/ARM64 builds remain outside this release's verification.