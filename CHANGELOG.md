# Changelog

## Unreleased

- Add per-user Linux desktop-menu installation, Unix browser-host registration, private same-user IPC with long-path/crash recovery, macOS app bundling and DPI-aware icons.
- Use Command shortcuts on macOS, enable AccessKit adapters and add asynchronous native folder pickers with protection against stale results.
- Add a real Unix CLI/native-host ranged-download smoke test and document tested Linux behavior and pending macOS acceptance.

- Produce a standalone `stratafetch-desktop.exe` alongside the Windows ZIP, and verify both artifact checksums and desktop binary identity.
- Preserve semicolons and escaped quotes in metadata filenames; honor UTF-8 extended filenames with language tags and fall back when their bytes are invalid.
- Show a paste-URL message instead of throwing when browser tab access fails or the current URL is unavailable; clear stale URLs.

## 0.1.0-alpha.3 — 2026-10-13

- Replace per-job queue polling with one notification-driven dispatcher.
- Stream immutable bounded IPC pages for read and mutation replies.
- Reject local staging corruption, size changes and redirected Windows destination folders before completion; extend fault tests.
- Prevent stale desktop snapshots and scheduler actions after configuration errors.
- Add visible browser handoff feedback, distinct Edge registration and ownership-safe unregister tests.
- Add actual local Windows queue/IPC/memory benchmarks and document incomplete OS/browser verification in `docs/PHASE4.md`.
- Rebuild portable staging cleanly and report installer copy/removal failures. Portable alpha release; installer runtime acceptance remains incomplete.

## 0.1.0-alpha.2 — 2026-10-13

Native desktop redesign: compact vector toolbar, navigation tree, virtualized sortable/resizable table, unified selection and keyboard input, async metadata/Add dialog, transfer tabs, organized settings/scheduler and both themes. Preserves engine/storage/CLI/browser behavior; adds paused-add and batch-remove commands. 43 local tests and authentic captures are documented in docs/PHASE3.md. Portable prerelease; unsigned/browser/DPI limits remain explicit. GitHub Actions stays disabled.

## 0.1.0-alpha.1 — development preview

Phase 2: public source repository, persisted global bandwidth/concurrency/connections, GUI per-job limits and priorities, category folders, timed starts, batch controls, safe JSON recipe export/import, improved typography/scrolling/branding, serialized destination reservations and queue admission, IPC blocking-work isolation/timeouts, native-host status/download reporting and registration cleanup, and a per-user NSIS installer. Full workspace tests, real crash-and-resume/native-host checks, streaming large-file measurements and isolated installer checks are documented in docs/PHASE2.md. Browser extensions and release acceptance remain experimental; Validation and packaging are local; GitHub Actions is disabled by request.

Rust streaming engine with SQLite checkpoints, verified ranges, safe fallback, local prefix and final-file SHA-256, pause/resume/restart/cancel, bounded concurrency, per-job bandwidth limits, native eframe/wgpu desktop, CLI and local IPC. Experimental browser handoff sources and portable packaging.

See docs/RELEASE-NOTES.md for the unsigned Windows alpha scope and docs/VERIFICATION.md for actual local checks.
