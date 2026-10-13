# Phase 4: Windows reliability work

This documents reliability work after `v0.1.0-alpha.2`, not a production-readiness declaration. The user subsequently explicitly requested publication of `v0.1.0-alpha.3`; it is distributed as an unsigned portable alpha with the remaining limitations below. Alpha.1/alpha.2 releases and assets are preserved. The native design, schema 2 history, download IDs, CLI and browser bridge remain in place. Builds, tests, lints, auditing and packaging were local. No workflow, hosted runner, billing configuration or paid service was added.

## Verified changes

- One engine dispatcher replaces one 25 ms admission-polling task per queued download. Waiting records have cancellation controls, but no individual task or timer. Admissions inspect record references without cloning/sorting the history. Completions, stops and settings changes notify the dispatcher; future schedules share one one-second timer. Priority/FIFO ordering and concurrency-cap regressions pass. Pausing a waiting record immediately removes its control, allowing edits/removal without waiting for a nonexistent transfer worker.
- IPC request protocol 2 streams immutable command-result pages on the same local connection: at most 128 records per page, approximately 512 KiB payload budget, still subject to the hard 1 MiB frame bound. Page sequence, completion and metadata consistency are checked; incomplete results are never returned. Mutation replies use the same pages. Version 1 requests retain small replies and receive an explicit upgrade error for oversized histories. Upgrade all three executables together; new clients require a new service. A real named-pipe test retrieves and configures all 4,001 persisted records beyond the old frame limit.
- Adding records no longer clones and sorts the existing history to reserve a filename or assign a sequence. Filename collision protection remains serialized.
- Before publication, staging size and segment hashes are compared against the bytes received from HTTP. Previously a fresh final checksum could record local damage as valid completion. The active-transfer corruption regression now fails without publishing a destination. Staging reparse attributes and redirected destination directories are rejected on Windows. An actual NTFS junction replacement test passes.
- The actual transfer write helper does not advance its digest/pending byte count after a partially successful write followed by an error. Injected Windows disk-full (112) and aborted-operation (995) errors exercise this helper. This is fault injection, not exhaustion of the user's disk. Durable checkpoints still follow successful file synchronization.
- Desktop polling responses started before/during a local mutation cannot replace its newer result. Scheduler command sequences stop at the first failed configuration rather than continuing to queue the download.
- Context-menu handoffs display a success/error badge and tooltip in Chromium and Firefox. Popup status is a live region, URL input has a label, and Send is disabled while its request runs. Permissions remain exactly nativeMessaging, contextMenus and activeTab: no cookies, host permissions, content scripts or interception.
- Registration supports distinct Chrome and Edge extension IDs. Matching-installation unregister fails closed if manifest ownership cannot be checked. An isolated real HKCU test verifies manifests, both origins, wrong-owner rejection, cleanup and history preservation without touching normal browser registrations.
- Portable staging is rebuilt from scratch under its verified workspace path, preventing stale package files. Build metadata derives the Cargo version and discloses a dirty working tree. NSIS stops on failed installation copies or executable removal and preserves the uninstall entry for retry. Installer runtime/upgrade behavior remains unverified in this phase.

## Checks actually performed

Windows reports version `10.0.26300.0`; Rust stable MSVC, release profile, locked offline dependencies, one Cargo compilation job.

| Check | Result |
| --- | --- |
| Formatting and strict all-target Clippy | Passed |
| Engine/app library, download, desktop-command and IPC tests | 44 Rust tests passed: 6 app library, 1 desktop-command, 2 IPC, 2 core library, 33 download tests |
| Extension script tests | 8 Node tests passed using mocked browser APIs; not installed-browser certification |
| Real registry test | Passed in a unique temporary HKCU branch; normal browser keys untouched |
| Locked dependency audit | 478 dependencies, 1,310 cached advisories, no reported vulnerabilities or warnings; cached database, not a fresh online fetch |
| Cargo deny | Advisories, bans, licenses and sources passed |
| Release application and examples | Built locally |
| Local ZIP and NSIS build | Fresh ZIP staging, all three executable hashes, required package files, source revision and both artifact checksums passed; NSIS compilation passed. These are development artifacts, not new release assets |
| Real native host / CLI / pipe smoke | Ping, add, origin rejection, unavailable-service errors and crash/resume passed |
| Crash-resume bytes | Service killed after 655,360 durable bytes; recovered paused, resumed to 8,388,608 bytes; SHA-256 `bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a` |
| Actual native rendering | Desktop exited successfully and captured [recovery frame](screenshot-phase4-recovery.png) |
| Entire workspace test command | Incomplete: Windows application control rejected CLI test wrapper before execution, error 4551 |
| Desktop's eight automated UI tests | Compiled but not executed: Windows application control rejected the desktop test executable, error 4551. Historical Phase 3 passes are not counted here |

The download suite covers successful ranged/fallback streams with byte comparisons, retries/server disconnects, malformed/truncated ranges, conflicting destination files, changed remote validators, modified/missing/truncated partial files, read-only Windows staging, redirected NTFS directories, priority/concurrency and history migration. Process termination is additionally tested with the real service. Real volume-full, ACL-denial, sudden storage loss and physical network-adapter changes were not induced.

## Actual UI and OS coverage

An isolated native session loaded 29 real persisted jobs. Actual input verified single selection, Ctrl+A selecting 29, double-click details with visible footer, Ctrl+comma Settings, light-theme switching, OK saving, Ctrl+N with URL focus, entering a localhost URL, and Download later adding a 30th paused record. Actual transfer progress changed and the 64 MiB fixture completed in the service. An independent SHA-256 comparison against generated fixture bytes and the persisted paused addition passed; see [native transfer evidence](PHASE4-NATIVE-TRANSFER.json). Open folder launched a new Explorer window titled `downloads`; Explorer inspection approval timed out, so its full path was not independently verified.

The UI Automation tree exposed only native window/title-bar controls, not the download controls. Screen-reader accessibility is therefore **not verified** and requires work; adding an AccessKit platform adapter is a remaining task. Keyboard shortcuts are verified, but full keyboard-only traversal is not.

Windows Settings launch returned `launched app did not expose a targetable window`. Actual OS display scaling at 100%, 125%, 150% and 200%, OS theme/high-contrast switching and mixed-DPI monitor movement were not tested. Historical egui scaling captures remain simulations. Resize/minimize, scheduler submission, sorting/filtering, context-menu activation, native file opening and preference persistence across a native restart were not completed in this phase. Sleep/resume and OS network disconnection were not performed. Physical Escape stopped computer use during the category-filter attempt; no subsequent UI input was issued.

Chrome was present in the computer-use app inventory; Edge and Firefox were not returned in that inventory (this does not prove they are absent from the machine). Actual extension installation and installed-browser end-to-end tests were not completed. Native framing/registration and mocked extension script checks are distinct evidence. Firefox permanent installation still requires Mozilla signing.

The existing installer test stopped before changes because an existing StrataFetch Start Menu folder must be preserved. No security settings were changed; no policy rejection was bypassed. Portable executables report `NotSigned`.

## Measured scalability

See [raw benchmark report](PHASE4-BENCHMARKS.json), `scripts/scalability.ps1` and the `scalability` example. These are actual local observations, not estimates or before/after comparisons. Each size uses fresh real SQLite records, reopens them, performs 20 direct-list and 20 whole-IPC-list operations, queues all jobs for a far-future time, samples the process every 200 ms during six seconds of idle waiting, then pauses all records. Seeding uses Store.save to exclude Manager.add setup cost. Start/pause timings include durable SQLite writes. No HTTP transfers or native GUI are in this benchmark. CPU/memory sample only the idle queue, not seeding or queue writes. An earlier run inadvertently sampled pause writes and was discarded; the published run ends the idle marker before those writes.

| Records | Peak working set, MiB | Peak private bytes, MiB | IPC list p95, ms | Direct list p95, ms | Start p95, ms | Pause p95, ms | Reopen, ms |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 11.72 | 5.49 | 2.51 | 0.20 | 0.97 | 0.92 | 17.15 |
| 1,000 | 14.17 | 8.00 | 7.04 | 0.99 | 0.98 | 0.88 | 115.54 |
| 10,000 | 31.37 | 26.27 | 65.72 | 11.71 | 1.02 | 0.94 | 1,282.87 |

All three idle samples observed **0 additional CPU seconds at Windows process-timer resolution**, over 5.72–5.80 seconds with 28 samples each. This is below measurable resolution in these short samples, not proof of zero CPU usage. The machine has 12 logical processors and other workloads were running. These results are one release build/run, not a broad hardware benchmark. Snapshot JSON reached 4,656,701 bytes at 10,000 records and was retrieved correctly through bounded frames.

Memory and UI work still scale with history length. The protocol bounds individual frames, not total history memory: server and client retain an O(n) immutable snapshot. GUI polling still retrieves complete snapshots, so large-history live CPU/network-pipe traffic and native frame latency need separate measurement. A changes-since-generation protocol and visible-range GUI caching remain possible later improvements. There is no claimed 10,000-row GUI responsiveness/FPS result here.

## Distribution/signing restrictions and release decision

The initial Phase 4 decision withheld alpha.3 because native regression/OS/browser tests and installer runtime validation were incomplete. Following the user's explicit request to publish a release, alpha.3 is published as a downloadable GitHub release, with its **alpha status** and those limitations disclosed. Only the verified portable ZIP and its checksums are release assets; the installer is withheld. No stable/production readiness claim is made. Previous releases and assets remain untouched.

Free legitimate signing possibilities were investigated, without enrollment, accounts, certificates, uploads or purchases:

- Microsoft documents Store MSIX certification/re-signing as a free signing route. This requires an MSIX package and Store submission; the existing ZIP/NSIS installer is not automatically signed. MSI/EXE Store submissions still require publisher signing. [Microsoft signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options).
- SignPath Foundation offers free signing for qualifying open-source projects, subject to its eligibility, reputation, verifiable-build and manual-approval rules. Compatibility with this project's strict local-only build policy has not been established, so no signing pipeline was configured. [Foundation conditions](https://signpath.org/terms).
- Self-signed certificates do not establish public Windows trust. Paid signing services/certificates were not purchased or enabled. Smart App Control requires a trusted provider's valid signature; unsigned binaries may be blocked. Do not disable protections, install a fabricated trust root, or mutate blocked binaries to evade policy. [Microsoft Smart App Control signing guidance](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control).

Completed-file verification can detect later user edits; arbitrary concurrent same-user filesystem tampering is not a security boundary. Publication validation adds a read of the recorded segment prefixes in addition to the final SHA-256 pass, trading disk I/O for detection of local damage. Hard-link publication remains required and will fail clearly on unsupported filesystems.

## Alpha.3 release-version verification

After the explicit request to publish, the workspace version was changed to 0.1.0-alpha.3 and rebuilt with the existing local toolchain. The full workspace suite then passed: **52 substantive Rust tests**, including all eight desktop UI regressions, plus empty wrapper/doc targets. Formatting, strict all-target Clippy and all eight extension-script tests passed. This supersedes the earlier blocked-test execution rows for the release-version test binaries; it does not complete the manual OS/browser checks or establish publisher trust. No Windows protection was changed or bypassed.

The final release build explicitly selected both applications and examples: cargo build --release --workspace --bins --examples --locked --offline -j1. A version check caught stale CLI/browser-host binaries left by an earlier examples-only build; they were rebuilt before the final smoke test or packaging. Package verification now rejects a CLI version different from Cargo.toml. The final alpha.3 CLI reports 0.1.0-alpha.3. The actual CLI/native-host/service crash-resume test then passed with 524,288 recovered bytes and the correct 8 MiB checksum; desktop rendering exited successfully. See ALPHA3-WINDOWS-SMOKE.json and screenshot-alpha3-recovery.png. Isolated browser registration/unregister passed again. Historical smoke/benchmark results above retain their original provenance.
