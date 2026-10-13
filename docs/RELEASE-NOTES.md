# StrataFetch v0.1.0-alpha.1

An experimental, unsigned Windows x64 release of the existing native Rust download manager. All validation and packaging run locally; GitHub Actions is disabled. This is not a production-ready release.

## Included

- Aggregate and per-download bandwidth limits, persisted concurrency and connection controls.
- Category folders, scheduled starts while the engine is running, priorities and batch pause/resume/cancel.
- Safe JSON download-recipe export/import; imports create paused jobs with fresh IDs.
- Improved native typography, scrolling details, search and original window/installer branding.
- Native host connection/download reporting, exact-origin checks and per-user registration cleanup.
- Portable ZIP and per-user NSIS installer, plus SHA256SUMS. Installer uninstallation preserves download state and completed files.

## Local evidence

Complete Windows workspace suite: 32 substantive tests, all executable wrappers and doc-test targets pass. Formatting, Clippy with warnings denied and dependency/advisory/license checks pass. Real CLI/native-host/IPC transfers, service-kill recovery and checksum verification pass. Native framebuffer captured from the working desktop. Five-cycle pause/resume and preview-schema migration are covered.

A deterministic 4 GiB file completed with 1.37 GiB free physical memory at start and about 18.8 MiB sampled CLI working-set peak. Independently generated and reread SHA-256 values match. This is a local workload measurement, not a universal speed or memory claim.

## Important limits

Binaries and installer are unsigned and have no verified publisher signature. Windows application control may reject unsigned executables; do not disable security protection to run them. Linux/macOS builds are not verified. Browser extensions are experimental: native host/IPC/registration were tested, but installed-browser end-to-end flows were not. Full native keyboard/high-DPI/accessibility review, disk-full/permission/power-loss faults, cross-user IPC denial and reparse-point races remain open. Large histories need IPC pagination and scalable queue dispatch.

Tray, notifications, autostart, background clipboard detection, drag/drop, configurable shortcuts, authenticated requests, mirrors and adaptive segmentation are not implemented. Scheduling requires a running engine and interrupted jobs reopen paused. Existing partial files retain their segment layout. Import/export contains URL query tokens; keep exports private. History import restores recipes, not trusted completed status or checkpoints.

This version migrates state from preview schema 1 to schema 2. Older preview binaries reject the upgraded database. Back up state before experimenting with downgrades. Read INSTALLATION.md, PHASE2.md and SECURITY-REVIEW-PHASE2.md for reproducible instructions and evidence.
