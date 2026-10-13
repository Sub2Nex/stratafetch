# Phase 3 — native desktop redesign

Windows x64, Rust 1.99 stable MSVC, 2026-10-13. Unsigned alpha; stable acceptance remains open. All builds, tests, audits and packaging run locally. GitHub Actions stays disabled. No hosted CI or billing configuration is involved.

## Implemented

The dashboard is replaced by a menu, original vector toolbar, compact resizable navigation tree, full-width download table and small status bar. Details, statistics, settings and scheduler appear on demand. Both themes use installed Segoe UI when available and existing egui fonts otherwise.

egui_extras 0.36.2 virtualizes 32-logical-pixel rows. Headers sort real values; columns resize, hide and reorder; Name fills spare width. Column layouts/widths, navigation/window geometry and preferences persist. One ID set handles single/Ctrl/Shift selection, Ctrl+A, arrows, Page Up/Down, Home/End, context menus and double-click details. Hidden rows leave selection. Enter/F2 opens details; completed-file opening is an explicit preference.

Add supports single/batch URLs, filename, folder, category and description. Async HEAD/header-only range probing never creates jobs. Download later creates a paused job; Start starts it. Transfer tabs show actual status/ranges, limits/priority and completion options. Session averages use observed progress during this GUI session. Completion messages/folder opening apply only to future completions while the desktop runs.

Settings preserve implemented engine controls and core English/Swedish labels; advanced text remains English. Descriptions/completion choices live in desktop preferences, not recipe exports. Existing SQLite schema 2, IDs, histories, checkpoints and Add/CLI/browser semantics are preserved. AddPaused and batch Remove are additive commands. Close older desktop/service instances before upgrading.

`desktop.rs` launches the app; `src/ui/` separates state, worker, theme/icons, toolbar, navigation, table and dialogs. Blocking engine work runs off the UI thread; one latest snapshot replaces buffered histories.

## Local evidence

- 43 substantive workspace tests, binary wrappers and doc-test targets pass. New tests cover HEAD/range metadata, paused addition, filtered/sorted selection, actual pointer/key events, 5,000-row virtualization and visible transfer/settings controls.
- Formatting and Clippy with warnings denied pass. Application/core release artifacts were cleaned and rebuilt with locked dependency caches and one compiler job.
- Refreshed cargo-audit reports zero known vulnerabilities/warnings; AUDIT.json records its database revision. cargo-deny passes advisories, bans, licenses and sources. Duplicate-version notices are informational. Notices cover 476 locked dependency packages.
- Real CLI/native-host/Windows IPC, service termination, paused recovery and resumed completion pass. Recovery retained 524,288 bytes. The independently checked 8 MiB SHA-256 is `bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a`.
- Genuine native framebuffers were inspected, corrected and recaptured: empty, one, mixed and 1,005 engine-created records, long names/URLs, failed/completed jobs, active transfers, themes and dialogs. Fixture contents are deterministic bytes; chosen extensions exercise icons. No image is a mockup.
- Dimension assertions cover 1280×720, 1366×768, 1920×1080 and 2560×1440. The oversized test window can extend beyond the monitor. 150%/200% captures simulate egui pixels per point; Windows display settings were not changed.
- Native interactions include single selection, Ctrl+A, Shift+Down, Ctrl+N, Ctrl+, and row context menus. Additional multi-selection/double-click behaviors have real egui input-event regression tests.

Inspection fixed inherited zoom/window sizes, an infinite transfer progress width, an unbounded settings divider, invisible controls, text-click interception and modifier-release timing. Every capture uses isolated UI persistence and waits for dialogs to finish appearing. PHASE3-CAPTURES.json records actual dimensions/counts.

## Reproduce locally

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -j 1
cargo build --release --workspace --locked -j 1
cargo build --release -p strata-app --example ui_fixture --locked -j 1
cargo build --release -p strata-core --example fixture_server --locked -j 1
node scripts/desktop-smoke.mjs
$env:STRATAFETCH_SMOKE_IMAGE = "$PWD/docs/screenshot-phase3-recovery.png"
node scripts/windows-smoke.mjs
```

The HTTP fixture uses port 18765. Scripts stop only their spawned processes and keep test data under ignored `.smoke`.

## Remaining limits

Binaries are unsigned. One earlier CLI build was rejected by Windows application control; the normal clean rebuild and real execution checks passed without disabling protection. Other machines may reject unsigned files. This prerelease includes a portable ZIP; no new installer roundtrip is claimed.

Actual Windows scaling/monitor transitions during transfers, full accessibility/keyboard review, every column persistence combination, OS file/folder associations and installed Chrome/Edge/Firefox flows need further manual review. Browser integration remains experimental. Linux/macOS, cross-user IPC denial, further disk/permission/power-loss faults and reparse races remain unverified.

Opening Windows Settings for the actual OS-scaling check was blocked by a Computer Use app-approval timeout. No display setting was changed. The recorded scaling captures remain layout simulations.

The table renders thousands efficiently, but the existing 1 MiB IPC frame limit and one polling task per queued job remain engine limits. The native large-history capture used 1,005 records; 5,000 rows are a rendering regression test, not unlimited remote history/queue support. Schedules require a running engine; interrupted work reopens paused. Stable release acceptance remains open.
