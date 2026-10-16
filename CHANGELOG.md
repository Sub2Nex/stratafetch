# Changelog

## Unreleased — extension 0.2.2

- Use persistent download identity and tolerate legitimate Chrome timestamp updates during browser capture.
- Wait briefly for an owned pause to become visible; continue to reject cancellation, manual resume, unsafe/private downloads and substituted URLs.
- Keep verified timestamps in recovery records and omit unconfirmed initial zero-byte sizes from native preflight metadata.
- Explain individual state/safety/identity/pause failures instead of a generic changed-download error.

## Unreleased — extension 0.2.1

- Preserve the real browser handoff failure reason and phase in the extension popup; redact HTTP(S) URLs and Windows paths, and clear the issue after success.
- Check executable locks before installer/uninstaller payload changes to avoid partially updating or removing an active installation.

## 0.1.0-beta.6

- Universal download/layers icon across executables, setup, native windows and extensions.
- Windows updates download and verify the complete installer plus update helper, keeping browser components together.
- Browser settings can repair the current package without a version downgrade; Windows instructions no longer mention Linux.
- Separate store listing copy, privacy answers, reviewer steps and branded media for Chrome, Edge and Firefox.


## 0.1.0-beta.5 — 2026-10-16

- Add confirmed automatic handoff of eligible browser downloads, with browser resume on decline/failure and interrupted-handoff recovery.
- Launch/focus one desktop window per profile and open transfer details for browser requests.
- Add guided local Chrome/Edge/Firefox setup and optional installer registration through native Windows APIs.
- Serialize simultaneous download confirmations, recheck queued downloads, retain failed-resume recovery records, and report actual native bridge readiness.
- Render confirmation in a focused StrataFetch window without an optional Linux message-dialog utility. Reject ordinary/older desktop exit codes as approval.
- Bundle stable local Chromium IDs, source ZIPs and store-upload ZIPs with icons and Firefox data declarations. Store signing/listings remain pending; local approval and Firefox temporary-loading limits remain explicit.
- Preserve unrelated native origins during register/unregister and reject mismatched companion components after desktop-only updates.
- Add optional domain exclusions, separate Chrome/Edge production-ID configuration, a dedicated Edge upload ZIP, unsigned Firefox development XPI, and a startup notice for older installed browser components.

## 0.1.0-beta.4 — 2026-10-16

- Retry disconnected/truncated range-probe bodies within the existing retry budget.
- Pace small writes before committing data under individual and global speed limits, keeping progress/checkpoints responsive and avoiding a large burst when paused.
- Retry briefly busy Windows local-service connections without replaying submitted commands.
- Discard queued desktop commands on exit instead of starting new work during shutdown.
- Reset transfer speed measurements after restart and release tracking/description state when history is removed.
- Repair Windows package/version checks for normal CRLF checkouts and record the actual Windows compiler target.
- Ship matching Windows setup, desktop/ZIP and Linux x64 assets built locally without CI or Actions.

## 0.1.0-beta.3 — 2026-10-16

- Add the recommended Windows setup wizard with versioned metadata, per-user installation, optional Start Menu shortcuts, finish-page launch and a Windows uninstall entry. Native install/reinstall/uninstall verification passes without removing unrelated data.

- Fix the Windows `TaskDialogIndirect` entry-point startup failure by embedding the Common Controls v6 application manifest in every executable, for MSVC and GNU builds.
- Reject Windows packages missing that embedded dependency during packaging.
- Publish a matching Linux x64 package alongside the Windows downloads.

## 0.1.0-beta.2 — 2026-10-16

- Desktop: automatic GitHub release checks, a manual Tools → Check for updates action, and a saved preference to disable automatic checks.
- Windows x64: download and verify a desktop update against release SHA-256 checksums, then install with graceful shutdown, restart and rollback if replacement fails. Other platforms link to the release downloads.
- Publish unsigned Windows x64 EXE/ZIP assets cross-built locally in Ubuntu WSL2. All 84 Rust tests, strict Clippy and 19 browser/CLI/native-host checks pass on Linux; the Linux settings window renders with the update preference. Native Windows execution and the full update/restart cycle remain unverified. No CI or Actions are used.

## 0.1.0-beta.1 — 2026-10-14

- Fix 18 reproduced engine, persistence, browser and desktop defects; see docs/BETA-AUDIT.md for regression coverage and scope.
- Preserve immediate pause/resume intent, CLI queue priority and native messaging through interrupted reads.
- Bound/decode Unicode filenames, escape Windows device aliases, retry initial probes and reject oversized IPC records before creation.
- Make history limits symmetric, publish private exports safely, protect Unix database permissions and refuse profile-lock bypass through database symlinks.
- Deliver worker events reliably and reject stale chooser/probe results; fix browser status races and single-stream resume/progress reporting.
- Upgrade Rustls to 0.23.46 for four newly published advisories and bump extension manifests to 0.1.1.
- Ship a Linux x64 public beta and unsigned Windows x64 preview. Native Windows/Mac, installed-browser and stable-release acceptance remain open.

## 0.1.0-alpha.4 — 2026-10-14

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
