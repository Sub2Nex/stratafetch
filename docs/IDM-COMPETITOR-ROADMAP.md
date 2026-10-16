# StrataFetch engineering roadmap

Updated 2026-10-16. Direct user priorities for this increment: universal icon, complete browser-store submission material and repair of the Windows desktop/component mismatch. The larger master engineering request is tracked below; unimplemented work is not represented as completed.

## Baseline audit

| Area | Existing implementation and evidence | Classification / remaining gap |
| --- | --- | --- |
| HTTP engine | network.rs validates range responses, validators/If-Range, identity encoding, byte boundaries, truncation/retry budgets; shared reqwest client; persisted segment hashes and final SHA-256. downloads/stability/beta_regressions integration suites exercise fallback/resume, validators, cancellation and disk faults. | Implemented with regression evidence; fixed segmentation needs improvement. |
| Allocation | Initial count is clamped by configured connections (1–16) and 256KiB minimum ranges. One worker per initial segment; no redistribution after a fast worker finishes. | Dynamic work allocation, adaptive splitting, per-host fairness and measured benchmarks missing. |
| Storage/history | Exclusive SQLite state ownership, private profiles, persisted jobs/settings, safe staging/publication, history import/export. Regression tests include Unicode/device aliases, URL limits, large history and permissions. | Implemented; broader power-loss/reparse/network-volume matrix needs improvement. |
| IPC/native bridge | Bounded framed messages, local origin allowlists, anonymous URL preflight, confirmation before network request, native data acknowledgment before browser cancellation. | Implemented with native and fixture evidence; store-installed end-to-end validation missing. |
| Extensions | Capture toggle, domain exclusions, serialized confirmations, context menu/manual URL, persistent background-owned port, interrupted-pause recovery and browser fallback. | Implemented with 117 JS fixture tests in beta.5; file type/min-size controls and explicit no-prompt mode missing. |
| Auth/media | Browser credentials/cookies are not copied; unsupported protected downloads fall back. | Scoped authenticated-download design, OS credential storage, HLS/DASH parsing/quality/FFmpeg merge missing. No DRM bypass planned. |
| Queue/scheduler | Priorities, max concurrent jobs, scheduled work while running, global/per-job rates, pause/resume/cancel and recovery to paused state. | Advanced profiles, persistent background scheduling, per-host limits, sleep/resume and batch templates need improvement. |
| Desktop | Native egui UI, themes, core Swedish labels, configurable columns, single instance/focus, confirmation/accessibility integration. | Full localization, tray, startup options, notifications, large-history performance measurements and more accessible flows need improvement. |
| Updates/packages | Per-user NSIS installer, Windows ZIP and Linux tar, native host setup, checksums, activation manifest. This increment adds the shared icon and full-setup Windows updates/current-version repair. | Full-setup native cycle must be verified for this increment; whole-package rollback after disk/installer failure remains a gap. |
| External services | GitHub releases available; publisher registered browser developer accounts. No assigned Chrome/Edge store IDs supplied. | Store draft upload/assigned IDs, origin rebuild, store review, Mozilla signing, legitimate Windows signing and real screenshots are external blockers. |

## Ordered milestones and acceptance gates

1. **Engine acceleration:** extract deterministic range planning; build a bounded work queue so idle workers claim remaining ranges; then investigate safe live splitting, adaptive concurrency and per-host fairness. Keep exact byte/validator/encoding invariants, cancellation and checkpoint consistency. Test uneven-speed fixtures, reconnects, remote mutation, ignored ranges and concurrency caps. Benchmark with fixed fixture bytes, latency, loss and hashes; compare browser/IDM only when actually accessible. No speed claims until measured.
2. **Browser experience:** finish store origins and actual Chrome/Edge/Firefox installation tests; add file-type/min-size controls and diagnostics; explicitly consented unattended mode only after failure recovery is proven. Design scoped authentication with protected storage separately. Never silently collect cookies.
3. **Non-DRM media:** isolated parsing/selection modules for direct MP4/WebM, HLS and DASH; lawful source fixtures, FFmpeg discovery/licensing, quality choice, cancellation/resume and temp cleanup. Reject protected streams rather than bypass restrictions.
4. **Queues/batch:** per-host fairness and bandwidth profiles, sleep/restart behavior, explicit startup preference, batch input/templates, duplicate/collision rules and import/export UX.
5. **Desktop polish:** measure large-list responsiveness, improve transfer details and keyboard/accessibility, implement full English/Swedish keys, tray and notifications with opt-in startup behavior.
6. **Integrity/security:** expected checksums, crash/power-loss and disk-full matrices, adversarial IPC/URL/path/redirect tests, reparse protection, log redaction and dependency review. Preserve security settings.
7. **Distribution:** transactional complete-package replacement/rollback, native upgrade/uninstall/profile-retention tests, store-signed browser builds and legitimate OS signing. Build locally without CI/Actions.
8. **Parity assessment:** publish a feature/evidence table, reproducible performance results and remaining unsupported scenarios. Do not claim IDM parity before the preceding gates pass.

## This increment

Prioritized Windows browser repair and release consistency before modifying engine concurrency. Replacing engine scheduling in the same branding/updater release would mix independent risks. Milestone 1 begins next with planner/work-queue fixtures; none of its missing capabilities or benchmark claims is marked complete here.
