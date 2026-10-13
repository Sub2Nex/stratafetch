# StrataFetch v0.1.0-alpha.2 — desktop redesign

An experimental, unsigned Windows x64 prerelease. All validation and packaging run locally. GitHub Actions remains disabled; no hosted CI or billing configuration is used. Alpha.1 is preserved.

## Changes

- Compact vector toolbar, navigation tree, full-width sortable table and small status bar replace the dashboard.
- Virtualized 32-pixel rows, configurable/resizable columns, unified Ctrl/Shift selection, keyboard navigation, context menus and double-click details.
- Async metadata checks, single/batch Add, paused Download later and on-demand transfer tabs.
- Organized settings, scheduler, measured statistics on demand, both themes and persisted preferences.
- Existing Rust engine, schema 2, IDs, checkpoints, CLI, IPC and browser handoff are preserved. Close older desktop/service instances before upgrading.

## Local verification

43 substantive workspace tests pass, including real egui pointer/key events and 5,000-row virtualization. Formatting, Clippy, refreshed vulnerability checks and dependency/license policies pass. Release crates were cleaned and rebuilt. Genuine native captures cover four requested resolutions, simulated 150%/200% layouts and 1,005 real records. CLI/native-host/IPC crash-and-resume transfers match the independently checked 8 MiB checksum. See PHASE3.md and PHASE3-CAPTURES.json.

## Downloads and limits

Portable ZIP and SHA256SUMS; no new installer. Binaries have no verified publisher signature. Windows application control may reject them; do not disable protection. One earlier CLI build was rejected locally; the clean rebuild and actual execution checks passed normally.

Browser integration remains experimental; installed-browser flows are unverified. Actual Windows display-scaling/monitor transitions and full manual accessibility/OS file-opening review remain open. Linux/macOS, further fault injection and cross-user IPC denial are unverified. IPC pagination and scalable queue dispatch still limit very large histories/queues. This is not a stable or production-readiness claim.

Scheduling requires a running engine; interrupted jobs reopen paused. Descriptions/completion choices are desktop preferences. Recipe imports create paused jobs with new IDs and do not restore trusted checkpoints/status. Exports may contain private URL query tokens; keep them private.
