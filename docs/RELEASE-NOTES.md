# StrataFetch v0.1.0-alpha.3 — Windows reliability improvements

Published unsigned Windows x64 alpha. The existing native UI, Rust engine, schema 2 history, CLI and browser bridge are preserved. All builds, tests, auditing and packaging are local; GitHub Actions remains disabled and no billing or hosted runner configuration was added. Previous releases remain available.

## Changes

- Replace one polling task per queued download with one notification-driven dispatcher and one shared schedule timer.
- Stream immutable bounded IPC pages, including mutation replies; verified beyond the old frame limit with 4,001 records. Close older desktop/service instances and replace all three executables together.
- Compare staging bytes against recorded segment hashes before completion; reject size changes, Windows staging reparse points and redirected destination folders.
- Extend partial-write/disk-full injection, missing/truncated staging, read-only Windows filesystem, active corruption and actual NTFS junction tests.
- Prevent stale desktop polling responses from replacing local command results; stop scheduler sequences after configuration errors.
- Show extension handoff success/error badges, improve popup accessibility, support distinct Chrome/Edge IDs and verify ownership-safe registration cleanup.
- Rebuild portable staging cleanly and improve installer failure reporting.

## Verified evidence

The alpha.3 release-version build passed the full workspace suite: 52 substantive Rust tests, including all 8 desktop UI regressions. All 8 mocked extension-script tests also passed. Formatting, strict Clippy, cached vulnerability auditing and dependency policies passed. Actual native-host/CLI/named-pipe crash/resume recovered an 8 MiB transfer byte-for-byte. A live native session completed a separate 64 MiB fixture with an independent SHA-256 match and added a paused download through the dialog. Isolated real HKCU registration/unregister tests preserved normal browser registration.

The actual 10,000-record release benchmark observed 31.4 MiB peak working set, 65.7 ms p95 whole-IPC-list latency and 1.02 ms p95 queue-start latency. Idle CPU was below Windows timer resolution in a short sample, not proven zero. The benchmark excludes GUI rendering and HTTP transfers. Raw methodology/results are in docs/PHASE4.md and docs/PHASE4-BENCHMARKS.json.

## Download and upgrade

Download stratafetch-windows-x64.zip and SHA256SUMS. Extract into a permanent folder, close the existing desktop and service, then launch stratafetch-desktop.exe. Default user history is outside the installation folder and schema 2 is preserved. Back up user state before upgrading; re-register the browser host if its path changes.

Only the portable ZIP is distributed. The installer compiles locally, but runtime install/upgrade/uninstall testing stopped to preserve an existing Start Menu installation; it is not offered as a release asset.

## Known limits

This is an alpha, not a stable or production-readiness claim. Binaries are unsigned; Windows application control may block execution. Do not disable Smart App Control or bypass security. Earlier Phase 4 test executables were blocked before execution. The legitimate alpha.3 version build subsequently ran the full suite successfully without security-setting changes; this does not establish trusted publisher signing.

Actual OS DPI settings, mixed-monitor transitions, high contrast, screen-reader support, sleep/resume, physical network changes and installed Chrome/Edge/Firefox flows remain unverified. UI Automation exposed only the native title bar. Manual testing was stopped with physical Escape; incomplete checks remain documented. Browser mocks and native-host tests do not prove installed-browser integration.

IPC bounds individual frames; full snapshots still use memory proportional to history, and GUI polling still requests full history. No 10,000-record GUI frame-rate result is claimed. Scheduling requires a running engine; interrupted transfers reopen paused. Publication requires hard links. Arbitrary same-user concurrent filesystem tampering is not a security boundary. Recipe exports can contain private URL query tokens.
