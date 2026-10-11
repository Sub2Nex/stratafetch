# StrataFetch

A native, local-first HTTP download manager built in Rust. MIT OR Apache-2.0.

**Development preview — release acceptance is still being verified.** No public release or performance advantage is claimed. There are no advertisements, telemetry, accounts, or cloud services.

## Run

Install stable Rust and the platform build tools. On Windows use Visual Studio Build Tools with the C++ workload. On Linux install `libxkbcommon-dev`, `libwayland-dev`, and standard C/C++ development tools.

```sh
cargo build --release --locked
cargo run --release --bin stratafetch-desktop
```

The desktop uses eframe 0.36.2 with wgpu. Paste a direct URL with Ctrl+V or open Add Download with Ctrl+N. You can paste several URLs, one per line. Set the destination in Settings or the Add dialog. Select a download to view segments, checksum, and transfer controls. Partial transfers remain paused after a restart. Completed files are never automatically executed.

## CLI

```sh
stratafetch add https://example.org/file.zip --directory ./downloads
stratafetch --json list
stratafetch status DOWNLOAD_ID
stratafetch pause DOWNLOAD_ID
stratafetch resume DOWNLOAD_ID
stratafetch cancel DOWNLOAD_ID
stratafetch restart DOWNLOAD_ID
stratafetch limit DOWNLOAD_ID 1048576
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
| Limits | Per-download limit through CLI; pause before changing |
| Native desktop | Real progress, measured speed, speed graph, search, status filters, details, context menus |
| Preferences | Destination, light/dark theme, core English/Swedish labels, window geometry |
| Browser handoff | Experimental Chrome/Edge/Firefox extensions and Rust native host; manual registration |
| Scheduling/categories/global bandwidth | Planned |
| Tray, notifications, clipboard monitoring, drag/drop, multi-selection | Planned |
| Authentication/custom headers/export | Planned; no embedded URL credentials accepted |
| HLS/DASH/DRM | Unsupported; direct MP4/WebM URLs work as ordinary files |

## Platforms and downloads

Windows x64 is the primary build target. Linux x64 and macOS are architecture/CI targets; support is only verified when their jobs pass. See [verification notes](docs/VERIFICATION.md) for actual local evidence. All generated binaries are unsigned. No signing or notarization is claimed. Portable packaging is provided; installer support remains planned.

## Browser integration

See [browser installation](docs/BROWSER.md). The extension reads only a selected link or explicitly requested current-tab URL. It does not collect cookies or browsing history, intercept all downloads, or detect protected streams. Start the desktop or CLI service before sending a URL.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

Tests use local servers and do not depend on public downloads. [Architecture](ARCHITECTURE.md), [security](SECURITY.md), [privacy](PRIVACY.md), [contributing](CONTRIBUTING.md), and [roadmap](ROADMAP.md) describe the design and remaining work. The compact workspace separates the reusable engine from application clients, with networking and storage in separate modules.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE), at your option. Contributions use the same dual license.
