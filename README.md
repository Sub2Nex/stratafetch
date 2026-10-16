# StrataFetch

A native, local-first HTTP download manager built in Rust. MIT OR Apache-2.0.

**Public testing beta — Linux x64 is validated; Windows x64 is an unsigned cross-build preview. Stable release acceptance remains open.** No production readiness or performance advantage is claimed. There are no advertisements, telemetry, accounts, or cloud services.

Source: [Sub2Nex/stratafetch](https://github.com/Sub2Nex/stratafetch). [Releases](https://github.com/Sub2Nex/stratafetch/releases). [Desktop redesign and local evidence](docs/PHASE3.md) describe the locally tested alpha and its remaining limits.

![Actual redesigned StrataFetch desktop with a real active transfer](docs/screenshot-phase3-dark.png)

![Actual light theme](docs/screenshot-phase3-light.png)

![Actual on-demand transfer details](docs/screenshot-phase3-transfer.png)

## Run

For a Windows portable preview, extract the ZIP and run `stratafetch-desktop.exe`. The command-line executable is `stratafetch.exe`. Rust is only needed when building from source.

Install stable Rust and the platform build tools. On Windows use Visual Studio Build Tools with the C++ workload. On Linux install `libxkbcommon-dev`, `libwayland-dev`, and standard C/C++ development tools.

```sh
cargo build --release --locked
cargo run --release --bin stratafetch-desktop
```

The desktop uses eframe 0.36.2 with wgpu. Paste a direct URL with Ctrl+V or open Add Download with Ctrl+N. You can paste several URLs, one per line. Set the destination in Settings or the Add dialog. The compact toolbar and navigation tree surround a sortable, resizable 32-pixel download table. Single-click selects; Ctrl-click toggles; Shift-click selects a range; Ctrl+A selects the current view. Double-click or Enter/F2 opens transfer details. Use View to choose/reorder columns or hide navigation. Metadata checks read headers without creating a job; Download later creates a paused job. Statistics, settings and scheduler open on demand. Partial transfers remain paused after a restart. Completed files are never automatically executed. Native rendering, selected interactions and local input-event regression tests are verified. See PHASE3.md for remaining manual DPI/accessibility checks.

## Updates

Download [v0.1.0-beta.3](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.3): [Windows desktop EXE](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.3/stratafetch-desktop.exe), [Windows portable ZIP](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.3/stratafetch-windows-x64.zip), [Linux x64 archive](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.3/stratafetch-linux-x64.tar.gz), and [SHA256SUMS](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.3/SHA256SUMS). Beta.3 fixes beta.2's Windows `TaskDialogIndirect` startup failure. Windows desktop startup and the isolated updater-helper replacement/restart path are now verified on Windows. Existing users of the broken beta.2 desktop must download beta.3 manually. Binaries remain unsigned, and Chrome may perform its own safety scan. All builds run locally without CI or Actions.

The desktop supports automatic update checks and **Tools → Check for updates…**. Windows x64 users can download, verify and install desktop updates with a restart. Automatic checks can be disabled in Settings → General. See [update behavior and release packaging](docs/UPDATES.md).

## CLI

```sh
stratafetch add https://example.org/file.zip --directory ./downloads
stratafetch --json list
stratafetch --require-service list
stratafetch status DOWNLOAD_ID
stratafetch pause DOWNLOAD_ID
stratafetch resume DOWNLOAD_ID
stratafetch cancel DOWNLOAD_ID
stratafetch restart DOWNLOAD_ID
stratafetch limit DOWNLOAD_ID 1048576
stratafetch settings --connections 4 --concurrent-jobs 3 --global-bytes-per-second 1048576 --organize-by-extension
stratafetch schedule DOWNLOAD_ID UNIX_UTC_TIME --category Work
stratafetch export history.json
stratafetch import history.json --directory ./downloads
stratafetch verify DOWNLOAD_ID EXPECTED_SHA256
stratafetch serve
```

With a desktop or `serve` instance running, commands use local IPC. Without one, the CLI opens the state directory exclusively and runs the transfer until completion or Ctrl+C. `--state PATH` isolates another instance. Removing a job deletes history and staging data, preserving completed files. Pause before changing a limit or removing a running job.

## Feature status

| Feature | Implementation |
|---|---|
| HTTP/HTTPS, redirects, TLS validation | Streaming reqwest/rustls |
| Byte-range transfers | Up to four connections by default; exact response bounds and validators checked |
| Fallback | Single stream when ranges or validators are unavailable |
| Pause/resume | Persisted segment checkpoints; safe resume requires matching remote validators and local prefix checksums |
| Recovery | Interrupted jobs become paused; durable bytes precede checkpoint writes |
| Integrity | SHA-256 partial-prefix and completed-file checksums; GUI/CLI can re-read completed files for verification |
| Concurrency | Three active jobs by default; queued jobs ordered by priority |
| Limits | Aggregate bandwidth through GUI/CLI; per-download GUI/CLI limits and priority; pause before changing job settings |
| Native desktop | Virtualized sortable table, unified selection, original vector toolbar, tree filters, async Add dialog, tabbed transfer details; measured statistics on demand |
| Preferences | Destination, light/dark theme, core English/Swedish labels, window/sidebar/table widths and column order, completion options |
| Browser handoff | Experimental Chrome/Edge/Firefox extensions and Rust native host; manual registration |
| Scheduling/categories | Scheduled UTC starts while engine runs; categories, optional extension subfolders, priority/FIFO queues |
| Batch controls/history recipes | GUI batches pause/resume/cancel/remove in groups of 256; bounded JSON exports/imports recreate paused recipes |
| Tray, notifications, clipboard monitoring, drag/drop | Planned |
| Authentication/custom headers | Planned; no embedded URL credentials accepted |
| HLS/DASH/DRM | Unsupported; direct MP4/WebM URLs work as ordinary files |

## Comparison scope

| Capability | StrataFetch preview | Established managers |
|---|---|---|
| Resume and bandwidth controls | Byte-validated HTTP resume and payload limits | FDM documents resume/traffic modes; JDownloader documents pause and bandwidth limits |
| Organization and scheduling | Extension folders and timed starts while running | IDM and FDM document broader category/scheduler options |
| Browser integration | Explicit handoff, experimental extensions | IDM documents mature browser integration and site exceptions |
| Archive extraction/torrents | Not implemented | JDownloader documents archive extraction; FDM documents BitTorrent |
| Public source | Entire Rust workspace, MIT OR Apache-2.0 | JDownloader describes open source; FDM labels open source specifically under legacy 3.9 features |

This compares documented scope, not independently tested speed, reliability, or security. Sources checked 2026-10-11: [IDM options](https://www.internetdownloadmanager.com/support/options.html), [FDM features](https://www.freedownloadmanager.org/features.htm), [JDownloader overview](https://jdownloader.org/download/index).

## Platforms and downloads

Beta.1 publishes Windows x64 and Linux x64 packages, with Linux desktop-menu and browser-host integration. The Windows binaries are cross-compiled and have not been executed on Windows. The Linux build was verified on Debian 13 and requires GLIBC_2.39. macOS app bundling and OS integration are prepared, but native Mac builds/tests remain pending and no Mac binary is published. See [platform setup and evidence](docs/PLATFORMS.md). All testing and packaging run locally; GitHub Actions is disabled. All binaries and the historical per-user NSIS installer are unsigned. No signing or notarization is claimed. Stable release acceptance remains open.

Download [v0.1.0-beta.1](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.1): [Windows desktop .exe](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.1/stratafetch-desktop.exe), [Windows portable ZIP](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.1/stratafetch-windows-x64.zip), [Linux x64 archive](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.1/stratafetch-linux-x64.tar.gz) and [SHA256SUMS](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.1/SHA256SUMS). The executable is portable; no installer is included. Browser integration is a separate opt-in step. [Packaging instructions](docs/INSTALLATION.md) describe native builds; beta.1's Windows cross-build is disclosed in the [release notes](docs/RELEASE-NOTES.md).

History exports contain URLs, including any query tokens. Keep them private. Imports do not restore checkpoints or completed status and never start transfers automatically. Schedules need a running desktop/service and reopen paused after shutdown. Engine settings persist in SQLite; existing partial files keep their segment layouts.

## Browser integration

See [browser installation](docs/BROWSER.md). The extension reads only a selected link or explicitly requested current-tab URL. It does not collect cookies or browsing history, intercept all downloads, or detect protected streams. Start the desktop or CLI service before sending a URL.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

Tests use local servers and do not depend on public downloads. `node --test scripts/unix-smoke.mjs` additionally exercises the real Unix CLI/native-host download flow after a debug build. Linux/macOS build, packaging, native folder dialogs and per-user integration are documented in [PLATFORMS.md](docs/PLATFORMS.md). [Architecture](ARCHITECTURE.md), [security](SECURITY.md), [privacy](PRIVACY.md), [contributing](CONTRIBUTING.md), and [roadmap](ROADMAP.md) describe the design and remaining work. The compact workspace separates the reusable engine from application clients, with networking and storage in separate modules.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE), at your option. Contributions use the same dual license.
## Beta development audit

Published v0.1.0-beta.1 includes 18 reproduced bug fixes and a patched TLS dependency. See [BETA-AUDIT.md](docs/BETA-AUDIT.md) for regression evidence, beta scope and remaining platform checks. The Releases page is authoritative about publication.

## Phase 4 development verification

Reliability changes after alpha.2 and actual local benchmark results are documented in [docs/PHASE4.md](docs/PHASE4.md). [Download v0.1.0-alpha.3](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-alpha.3). This unsigned portable alpha includes the reliability fixes; incomplete native/browser checks and Windows application-control restrictions remain documented. All build and verification work remains local, with no GitHub Actions or billing configuration.
