# Beta audit and release scope

The 2026-10-14 audit found and fixed 18 reproducible defects in the alpha.4 source. Each fix has a regression check; the original failures were captured before the changes. The audit used an archived source snapshot for the normal Unix permission checks. Download fixtures use loopback HTTP and isolated profiles, with exact bytes and SHA-256 comparisons.

| Defect | Result after the fix | Regression coverage |
| --- | --- | --- |
| CLI/native-host filenames retained URL percent escapes | Decode UTF-8 URL filenames consistently with the desktop | `beta_regressions::url_filename_is_decoded_for_cli_and_browser_adds` |
| Long Unicode filenames exceeded filesystem byte limits | Bound names to 180 UTF-8 bytes, preserve short extensions and character boundaries | `unicode_filenames_fit_the_real_filesystem` |
| Windows device aliases could be used as filenames | Escape superscript COM/LPT aliases, console device names and reserved stems with spaces | `windows_device_aliases_are_not_download_names` |
| Exported histories over 1,000 entries could not be imported | Use a shared 10,000-entry limit and retain the 8 MiB byte limit | `histories_over_one_thousand_entries_roundtrip` |
| Exported Unicode category labels could not be imported | Normalize labels and filenames to the same safe byte bounds | `unicode_history_categories_roundtrip` |
| Initial HTTP probes bypassed retries | Apply the retry budget and Retry-After handling to probes; cancellation interrupts waits | `transient_probe_failures_honor_the_retry_budget` |
| Resume immediately after Pause was silently ignored | Queue the resume intent and admit it atomically when the old worker ends; a later Pause cancels it | `immediate_resume_after_pause_is_not_lost`, `pausing_again_cancels_a_pending_immediate_resume` |
| Very large URLs created records that IPC could not return | Reject URLs over 16 KiB before creating directories or records | `oversized_urls_do_not_create_unreadable_ipc_records` |
| Unix history exports inherited readable permissions | Write a private temporary file and publish without overwriting an existing export | `private_history_exports_protect_signed_url_parameters` |
| Unix database and SQLite sidecars inherited readable permissions | Create private state directories/files and tighten existing database/sidecar permissions | `state_database_is_private_even_with_a_permissive_umask` |
| Database symlinks let separate profile locks address the same database | Refuse database/lockfile links and non-regular state entries; inspect Windows reparse entries | `state_database_symlinks_cannot_alias_another_profile` |
| CLI rate changes silently reset queue priority | Change the rate while preserving priority under the engine control lock | `cli_regressions::changing_rate_limit_preserves_queue_priority` |
| An interrupted native-message prefix read disconnected the bridge | Retry interrupted reads and retain exact framing/EOF validation | `native::interrupted_reads` |
| Bursts of worker events silently dropped metadata/folder results | Deliver accepted events reliably, while retaining the bounded command queue | `worker::tests::accepted_events_are_not_lost_when_a_batch_fills_the_queue` |
| Late popup startup pings erased current-tab errors | Apply only current status replies | `scripts/browser-tests.mjs`, both browsers |
| Badge API failures became unhandled handoff failures | Settle badge/title updates independently of native handoff | `scripts/browser-tests.mjs`, both browsers |
| Single-stream transfers claimed validated resume and displayed false segment lengths | Persist actual resumability and show accurate single-stream progress | `ui::tests::single_stream_transfers_do_not_claim_verified_resume` |
| Old chooser results modified reopened forms | Match each chooser to its originating form; metadata also uses form/probe generations | `ui_state::reopened_forms`, `async_result_order` |

Rustls was updated from 0.23.45 to 0.23.46 for newly published advisories RUSTSEC-2026-0346, 0347, 0348 and 0349. Optional Brotli/ECH/custom-cache configurations implicated by several advisories are not enabled here. The dependency was patched instead of adding audit exceptions. Dependency versions otherwise remain locked; the resolver selected an already locked getrandom version for tempfile. The engine now uses tempfile for private exports and libc on Unix for no-follow state-file opens.

## Beta acceptance

This is a public testing beta for the download engine and desktop on Linux x64. Windows x64 executables are unsigned GNU cross-build previews with runtime acceptance pending. macOS native builds require an Apple SDK and are not included. No stable release, full IDM parity, signing or installer acceptance is claimed.

Required checks: all 79 Rust tests; all 18 browser-script checks; formatting; strict all-target Clippy; dependency advisory/license/source checks; native Linux release build; real Unix CLI/native-host downloads; rejection of an untrusted HTTPS certificate; X11/Wayland rendering; matching packaged executable bytes, clean source metadata and uploaded SHA-256 digests. Preserve schema 2 and original profiles; upgrade all three executables together and back up state before testing.

The actual Chromium integration attempt failed because this cloud image's administrator policy reports: `Loading of unpacked extensions is disabled by the administrator.` This is a diagnosed environment blocker. Assertions were retained; the run is not counted as browser acceptance. The optional `scripts/chromium-smoke.mjs` can be run with `CHROMIUM_BIN` on a development machine that permits unpacked extensions. Chrome/Edge/Firefox installed-browser flows remain experimental.

Physical GPUs, distribution sessions, mixed DPI, accessibility clients, sleep/resume, Windows different-user ACL denial, malicious same-user filesystem replacement races, native Windows execution and native macOS builds remain open. Native chooser display was observed earlier, but full folder-selection acceptance remains pending. Linux packages built here require GLIBC_2.39. GitHub Actions and paid/hosted runners remain disabled.

[X11 rendering evidence](screenshot-beta-linux-x11.png) and [Wayland rendering evidence](screenshot-beta-linux-wayland.png) show a real completed 1 MiB beta fixture download with an independently verified checksum. Both use Mesa software Vulkan.

## Reproduce validation

Run from a source checkout of the release tag.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release --workspace --bins --locked
node --test scripts/browser-tests.mjs
STRATAFETCH_BIN_DIR="$PWD/target/release" node --test scripts/unix-smoke.mjs scripts/tls-smoke.mjs
cargo audit
cargo deny check
# Optional installed Chromium acceptance; it must execute, not skip:
CHROMIUM_BIN=/path/to/chromium node --test scripts/chromium-smoke.mjs
```
