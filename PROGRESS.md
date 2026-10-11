# StrataFetch engineering log

2026-10-11: Empty workspace inspected. Rust 1.99 MSVC available; GitHub CLI is unauthenticated. Creating Rust 2024 workspace. Engine correctness precedes desktop work. Initial architecture consolidates networking/storage/scheduling in strata-core and CLI/desktop in strata-app; separate modules will keep these boundaries explicit.

Release gate: do not tag 0.1.0 until engine tests, Windows binaries, and native launch have been verified. Secondary platforms require CI verification.

Engine implementation: safe streamed HTTP/HTTPS, strict range probe/response validation, 64-bit offsets, durable SQLite snapshots, local prefix hashes, publication-boundary recovery, cooperative pause/cancel, resume/restart, retries with jitter/Retry-After, bounded concurrency, FIFO/priority queues, per-job limits, final-file SHA-256 verification. Core is independent of GUI.

Desktop and clients: native eframe 0.36.2/wgpu application, live snapshots and statistics, search/status filters, virtualized rows, actual segment detail, themes/core Swedish labels, persistent preferences, background engine ownership or IPC attachment. CLI, protected Windows named pipes/Unix socket protocol, strict native messaging bridge, and experimental browser extensions implemented.

Final evidence: Windows optimized binaries built; 25 workspace tests pass; format/Clippy/docs pass; cargo-audit and cargo-deny pass. Native launch and actual live framebuffer verified. Desktop-owned slow transfer checkpointed at 2 MiB on exit; CLI resumed to exact 8 MiB fixture checksum. Practical HTTPS transfer succeeded. Ten local timing trials recorded without general speed claims.

Release infrastructure and documentation are present, including third-party notices and portable packaging. Source remains local: GitHub CLI/browser lack authentication and the connector cannot create repositories. Native manual UI automation timed out at helper approval; full manual review and secondary-platform/browser validation remain open. Do not claim stable 0.1.0 acceptance or publish a stable tag without those follow-ups.
