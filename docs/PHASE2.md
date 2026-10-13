# Phase 2 engineering and release decision

Windows x64, 2026-10-11. Continued the existing repository from e4ce03a35e20d95ef06fe1efa7aa7d11c84dd116. This remains an unsigned development preview. Public source: https://github.com/Sub2Nex/stratafetch. Stable release acceptance remains open. Locally validated Windows alpha packaging is documented in RELEASE-NOTES.md.

## Baseline and Windows failure cause

Exported baseline HEAD using git archive, extracted into .smoke/clean-source and built with a separate empty target/clean-verification directory and incremental compilation disabled. The complete workspace test invocation passed every target: 21 engine tests, four framing tests, one actual Windows IPC roundtrip, executable wrappers and doc tests (26 substantive tests). Nothing was skipped.

Windows Code Integrity Operational events 3033/3077 at 11:32:38 identify the previously blocked browser-host test executable and signing-policy rejection. Correlated event 3118 identifies Smart App Control. Policy ID 0283ac0f-fff1-49ae-ada1-8a933130cad6. This is evidenced application-control rejection, not an assumed antivirus detection. No security policy was changed or bypassed. Subsequent unsigned builds ran; acceptance on other machines is not guaranteed.

An initial restricted-sandbox test run failed IPC by timeout. The clean-source execution with local IPC/HTTP permission passed. This environment restriction is distinct from Code Integrity.

## Implemented download-management features

1. Persisted aggregate payload bandwidth limit through GUI/CLI. Includes retry payloads before disk writes; probes/headers and kernel buffers are outside accounting. 16 KiB quanta, no initial token burst, proportional timer waits capped at 20 ms. Timer granularity can underutilize high rates; fairness needs broader measurements.
2. GUI per-download bandwidth controls and priorities, with pause/wait required before mutation.
3. Runtime maximum simultaneous downloads, persisted in SQLite. Lowering the limit lets current jobs finish and prevents new admissions.
4. Connections for new transfers, persisted and bounded to 1–16. Existing partial files keep their verified segment layout.
5. Categories assigned from filename extensions; editable through the inactive-job scheduling panel; searchable with filename and URL.
6. Optional automatic category subfolders for new files. Canonical-path validation rejects category-folder redirection outside the destination.
7. Per-job UTC scheduled starts. The engine must stay running; interrupted queued jobs reopen paused. Future high-priority jobs do not block ready lower-priority jobs.
8. Priority/FIFO queue admission serialized with pause/cancel.
9. GUI multi-selection and pause/resume/cancel batches bounded to 256 IDs. Completed files are skipped; batch errors report affected IDs.
10. Bounded JSON history recipe export/import (8 MiB, 1000 imported entries). Explicit destination, safe HTTP URLs/names and fresh IDs. Imports stay paused and never trust checkpoint/checksum/completed state. Existing export files are preserved. I/O errors can leave an explicitly paused partial import.

Also improved typography/contrast, scrolling transfer details/settings, search, uncluttered batch toolbar and original native/installer icon geometry. Existing dark/light themes and Ctrl+N/explicit paste remain. IPC blocking verification/import work now runs off async I/O workers; response write timeout and handler error reporting added. The UI automation helper repeatedly rejected clicks because intervening user input was detected; a complete native control walkthrough is not claimed.

## Reliability fixes and tests

Fixed a simultaneous-add race where competing clients could reserve the same filename and durable sequence. Reservations are serialized and a sixteen-client regression verifies distinct destinations/sequences. Remove/configure/restart checks retain the control lock through mutation. Added repeated pause/resume regression (five cycles), persisted settings, aggregate-rate timing across two actual files, runtime concurrency changes, cancellation at 1 B/s, scheduled priority ordering, organization and import security tests.

The complete local workspace invocation passed 32 substantive tests: 27 engine integration tests, four framing tests and one actual IPC roundtrip, plus all executable wrappers/doc targets. Format and Clippy with warnings denied passed. cargo-audit found zero known advisories/warnings using the previously downloaded advisory database; cargo-deny advisories/bans/licenses/sources passed with informational duplicate-version warnings. No new runtime dependency was added. See SECURITY-REVIEW-PHASE2.md.

## Real executable and package evidence

scripts/windows-smoke.mjs exercised actual release CLI, native host and Windows IPC with isolated state. Native ping/add, accepted download filename/status, unregistered-origin rejection and service-unavailable errors passed. Killing the service mid-transfer left 720,896 durable bytes, recovered as Paused. Standalone resume produced 8,388,608 correct bytes and independent SHA-256 bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a. The native desktop rendered that real completed transfer and exited 0 after its own framebuffer capture (screenshot-phase2.png).

Windows registration wrote actual per-user Chrome/Edge/Firefox keys and UTF-8 manifests. Unregistration removed the test keys and three origin/host manifests, preserving download state. Actual installed browser extension flows are still unverified and experimental; no interception claim is made.

NSIS 3.13 built a real per-user installer alongside the portable ZIP. An isolated silent install/uninstall test verified the installed desktop binary, product-file/registry cleanup and preservation of an unrelated sentinel file. Optional Start Menu shortcuts and matching-browser-registration cleanup are included. No elevation, signing, autostart or automatic browser installation is claimed. Interactive page/default-path/upgrade review remains open.

The streaming fixture generated 1 GiB and 4 GiB without materializing them in RAM. The 4 GiB transfer ran with 1,473,359,872 bytes free physical memory at start, sampled peak CLI working set 19,693,568 bytes, 81.801 seconds and exit 0. A 1 GiB transfer sampled 19,615,744 bytes peak, 11.234 seconds. Independently generated fixture hashes and reread file hashes are recorded in large-transfer-phase2.json and large-transfer-4g-phase2.json. These are sampled working sets, not allocator bounds or universal throughput claims.

Ten updated 8 MiB loopback trials and matching hashes are in benchmark-phase2.csv. The fixture now streams 64 KiB chunks, so comparison against the earlier whole-response fixture is not equivalent. No IDM acceleration, adaptive segmentation or generalized optimization claim is made.

## Publication and remaining limits

Created and verified the public source repository, main branch and description/topics. First Phase 2 pushed commit: f55f6cb913241257ed203f56efd871ff3c929cec. On 2026-10-13 the user requested local validation only. GitHub Actions was disabled through repository settings and workflow/automated-update configuration removed. Hosted CI is not a release requirement. Linux/macOS remain unverified; Windows has local evidence only.

An experimental Windows alpha may be published after local alpha package validation; the remaining gates apply to stable release acceptance. RELEASE-CHECKLIST.md records the decision. Remaining priorities include full native interaction/high-DPI/accessibility review, installed-browser flows, disk-full/permission/power-loss faults, cross-user IPC denial, reparse-point TOCTOU hardening, installer upgrades, large-history IPC pagination/scalable queued-task dispatch, tray/notifications/autostart/opt-in clipboard/drag-drop/onboarding, configurable shortcuts, completion actions, custom headers/authentication/proxy UI, mirrors/adaptive segmentation and configurable retry/timeout policies.

Final parallel Clippy/build encountered Windows paging-memory exhaustion (OS error 1455), causing cascading metadata-load errors. With one Cargo build job, Clippy passed. Only the two independently verified large fixture files were removed after validating their absolute paths under .smoke/large-output; reports/hashes remain. No OS paging or security setting was changed.

2026-10-13 local alpha validation: all 32 tests passed, advisory database refreshed to 64a0302dc836df15a10ac27ebdcdf8f9b4ce845a (1309 advisories, zero findings), and actual v0.1.0-alpha.1 native-host/CLI/desktop smoke passed. A fresh service-kill check recovered 524,288 durable bytes, then resumed to the same independently verified 8 MiB digest. Schema 1 to 2 migration and category backfill are tested. The landing page responded HTTP 200 and its actual browser DOM/layout was inspected locally; no hosted build is required or invoked. Stable-release gates remain open and are disclosed in the alpha notes.

Final alpha package verification, 2026-10-13: all ZIP executables matched the release build, required files and source revision d8cda2737db90c9776c82f6018b0987c27634040 matched, and both artifact checksums passed. The unsigned final installer was blocked by Windows application control before installation; the earlier preview roundtrip passed, but the final installer roundtrip is not claimed. No protection or policy was changed. The portable application execution checks passed. Release notes explicitly distinguish these results.
