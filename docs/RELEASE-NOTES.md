# StrataFetch v0.1.0-alpha.4 — native platform integration

Experimental desktop prerelease with download metadata fixes, Unix integration and a directly downloadable Windows desktop executable. Stable release acceptance remains open. Validation and builds are local; no GitHub Actions, paid runners or signing services are used.

## Changes

- Fix quoted Content-Disposition filenames, UTF-8 language-tag parameters and invalid UTF-8 fallback.
- Handle unavailable browser-tab URLs without unhandled errors or stale download URLs.
- Add per-user Linux application-menu/icon installation and ownership-safe removal.
- Register Chrome, Chromium, Edge and Firefox native hosts on Linux/macOS, preserving other installations and history.
- Use private same-user Unix IPC sockets that handle long state paths, stale sockets and normal shutdown cleanup.
- Add native destination folder dialogs, macOS Command shortcuts, accessibility adapters and high-DPI vector icons.
- Prepare native macOS .app bundles; Mac builds and runtime acceptance remain pending.
- Package a standalone Windows desktop .exe alongside the full ZIP and verify artifact checksums.

## Downloads

- **stratafetch-desktop.exe**: standalone Windows x64 desktop preview, built with the GNU Windows toolchain from Linux. It is portable, not an installer. This new Windows build has not been executed on Windows; retain alpha status and report failures.
- **stratafetch-windows-x64.zip**: matching desktop, CLI and native browser-host executables, extensions, documentation and dependency notices. Upgrade all three executables together. Binaries are unsigned.
- **stratafetch-linux-x64.tar.gz**: locally verified Debian 13 x64 desktop/CLI/native-host build, with X11 and Wayland support. The desktop references GLIBC_2.39; older distributions require a native rebuild.
- **SHA256SUMS**: SHA-256 for every binary/archive asset above.

No installer, macOS binary or Linux ARM64 binary is included. macOS packaging is present in the source, but an Apple compiler/SDK and native validation are required before shipping Mac binaries.

## Validation and remaining checks

The alpha.4 Linux source/build passed all 60 Rust tests, 14 mocked browser-extension tests, formatting and strict all-target Clippy. The real Unix CLI/native-host service smoke check downloads from a local range-capable server and independently checks all bytes and SHA-256. Linux native rendering was exercised with X11 and Wayland using software Vulkan; this does not certify physical GPUs or distribution desktop sessions.

Windows executables are cross-compiled with checksum/package and PE import inspection. That is build evidence, not Windows runtime acceptance. Native Windows regression tests, installed-browser integration, Mac Intel/Apple Silicon builds, DPI/monitor transitions, accessibility clients, sleep/resume and full folder-selection acceptance remain pending. The cloud portal denied process-root inspection; its GTK fallback displayed a chooser, but the automated expected-directory assertion did not pass. See docs/PLATFORMS.md for precise evidence and limitations.

Default history remains outside the installation directory and schema 2 is preserved. Back up state before upgrading and unregister/re-register browser hosts if their permanent path changes. Completed files are not automatically executed. History exports may contain private URL query parameters. Unsigned binaries do not establish trusted publisher signing or notarization.
