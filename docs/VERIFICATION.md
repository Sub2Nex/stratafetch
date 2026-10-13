# Verification record

The current desktop redesign and 43-test local record are in [PHASE3.md](PHASE3.md). The material below is historical Phase 1/2 evidence; its old publication blockers and counts do not describe current source.

Phase 2 supersedes the preview results below. See [PHASE2.md](PHASE2.md) for clean-source workspace tests, evidence identifying Smart App Control, implemented controls, real process-kill/native-host checks, installer checks and publication blockers. All 32 substantive tests currently pass in a complete workspace invocation. The source is public at https://github.com/Sub2Nex/stratafetch. GitHub Actions is disabled by request; local validation is the release standard. Linux/macOS remain unverified. Stable release acceptance remains open.

Environment: Windows x64, Rust 1.99.0 stable MSVC, 2026-10-11. This is an unsigned development preview, not an accepted stable release.

## Passed checks

- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --offline -- -D warnings`: no source lint failures. The sandbox occasionally reports incremental-cache hard-link copy warnings.
- Earlier `cargo test --workspace --offline`: 25 tests passed. After final range-bound hardening, all 26 substantive tests passed through `cargo test -p strata-core --offline` (21 engine integration tests) and `cargo test -p strata-app --lib --test ipc --offline` (four framing tests plus one Windows IPC roundtrip). The final full-workspace invocation was interrupted by Windows application control blocking a zero-test browser-host wrapper; no failing assertion occurred.
- `cargo build --release --workspace --offline`: all three Windows MSVC x64 executables built.
- `cargo doc --workspace --no-deps --offline`.
- cargo-audit 0.22.2: zero known vulnerabilities, zero advisory warnings. Advisory database commit and results are recorded in AUDIT.json.
- cargo-deny 0.20.2: advisories, bans, licenses, and sources passed. Duplicate transitive-version warnings remain informational. Published packages retain their original notices; see DEPENDENCIES.md.
- Chromium/Firefox JavaScript syntax and manifest JSON parsing.

## Actual application checks

The release CLI downloaded the 8 MiB local fixture with four verified segments. Its final digest matched an independently generated expected digest:

`bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a`

The native wgpu desktop launched, displayed real persisted data, captured its actual framebuffer, and exited with code 0. A second native-owned transfer was submitted through the desktop's local service. Its live transfer appears in screenshot.png. Graceful desktop shutdown persisted Paused at 2,097,152 / 8,388,608 bytes. The standalone CLI resumed that job to completion and re-read the file to verify the expected SHA-256.

A real HTTPS transfer from https://www.rust-lang.org/ completed with normal certificate validation, producing an 18,594-byte HTML response. This practical smoke check is separate from the deterministic test suite.

Windows application control intermittently rejected unsigned desktop/test executables with error 4551. Native builds subsequently launched normally, an earlier complete workspace run passed, and final substantive test targets passed independently. The latest full-workspace attempt hit the policy block on a zero-test browser-host wrapper. No security setting was changed or protection bypassed. Binaries remain unsigned and may require approval under other machines' policies.

Ten measured loopback timing trials are recorded in BENCHMARKS.md and benchmark.csv. No WAN, browser/IDM, CPU, or memory-performance claim is made.

## Remaining verification

Full manual UI input/keyboard/Open File flows, different-user Windows IPC denial, browser installation/native-host end-to-end testing, Linux/macOS builds, disk-full fault injection, and additional process-kill scenarios remain outstanding. The native UI automation helper's application-approval request timed out; manual interactions are not claimed as verified. Initial release acceptance remains open.

GitHub CLI and the available browser session are unauthenticated. The authenticated GitHub connector lacks repository-creation tools. No public repository, release tag, or GitHub release was created. See PUBLISH.md for exact future publication commands.
