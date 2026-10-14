# StrataFetch v0.1.0-beta.1 — first public testing beta

Beta for the download engine and Linux desktop, with an unsigned Windows x64 cross-build preview. This release fixes 18 reproduced alpha.4 defects and upgrades Rustls for four newly published security advisories. Stable and full platform acceptance remain open. All validation and packaging are local; GitHub Actions and hosted runners remain disabled.

## Fixes

- Preserve an immediate Resume after Pause, retain queue priority when setting a CLI rate limit, and cancel pending resume requests when paused again.
- Decode CLI/browser URL filenames; bound Unicode names by bytes; preserve short extensions; escape additional Windows device aliases.
- Retry transient initial HTTP probes and interrupt retry/hash waits when paused.
- Reject oversized URLs before creating records that exceed IPC limits.
- Make exported histories importable above 1,000 entries with a shared 10,000-entry/8 MiB limit; normalize Unicode labels; publish private exports without overwriting files.
- Restrict Unix database/sidecar permissions and refuse database symlink aliases that bypass profile locking.
- Preserve native messaging through interrupted reads and deliver worker events without dropping metadata/folder results.
- Prevent late browser replies from replacing current errors, and handle action-badge failures without losing native handoff results.
- Show actual single-stream resumability/progress and ignore stale native chooser/metadata results from old forms.
- Upgrade Rustls to 0.23.46 for RUSTSEC-2026-0346, 0347, 0348 and 0349.

## Downloads

- **stratafetch-desktop.exe**: standalone Windows x64 desktop preview. Portable, not an installer.
- **stratafetch-windows-x64.zip**: matching Windows desktop, CLI, native host, extensions, documentation and dependency notices. GNU cross-compiled on Linux; not executed on Windows.
- **stratafetch-linux-x64.tar.gz**: verified Debian 13 x64 desktop/CLI/native-host package. Requires GLIBC_2.39 and an X11/Wayland session with a supported Vulkan driver.
- **SHA256SUMS**: hashes for all executable/archive assets above.

No macOS/ARM64 binary or installer is included. macOS source integration and app packaging are prepared; native build/runtime acceptance requires Apple's SDK. Binaries are unsigned.

## Evidence and limits

All 79 Rust tests and 18 browser-script checks pass, including the new regressions. Formatting, strict Clippy, dependency advisories/licenses/sources, release CLI/native-host exact-byte/SHA-256 downloads and rejection of an untrusted HTTPS certificate pass. Native Linux rendering is validated with X11 and Wayland using software Vulkan. Archive executable bytes, source metadata and uploaded asset hashes are verified before publication.

Installed Chromium testing was blocked by this environment's administrator policy forbidding unpacked extensions. That failed attempt is documented, not counted as acceptance. Browser integration remains experimental. Native Windows/Mac execution, physical GPUs, installed browsers, full folder-selection acceptance, accessibility/DPI transitions, sleep/resume, signing and different-user Windows IPC checks remain pending. See [the beta audit](https://github.com/Sub2Nex/stratafetch/blob/v0.1.0-beta.1/docs/BETA-AUDIT.md) for every fix, regression and limitation.

Back up profiles and close old desktop/service instances before upgrading all three executables together. Schema 2 is preserved; interrupted jobs remain paused. Re-register the native host if its permanent path changes. Completed files are never automatically executed. Exports contain URLs and may contain private query parameters.
