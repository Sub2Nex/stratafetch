# Reproducible release checklist

- [x] Build and test the baseline from `git archive HEAD` with a fresh target directory.
- [x] Run complete workspace tests, including binary wrappers and doc tests.
- [x] Run formatting, Clippy with warnings denied, cargo-audit and cargo-deny.
- [x] Verify actual segmented/fallback bytes, malformed ranges, retries and collisions.
- [x] Verify repeated pause/resume, real service termination, persisted recovery and full checksum after resume.
- [x] Verify runtime aggregate/concurrency controls, persisted settings and scheduled priority ordering.
- [x] Verify native desktop launch/framebuffer and actual CLI/native-host/IPC downloads.
- [x] Build portable ZIP and per-user NSIS installer; test isolated install/uninstall and preserve unrelated data.
- [x] Publish public source with original branding, licenses, build docs and workflows.
- [ ] Perform full native control/keyboard/high-DPI/accessibility walkthrough and light/dark layout review.
- [ ] Perform actual Chrome/Edge/Firefox extension-to-application flows, or retain explicit experimental status.
- [ ] Fault-inject disk full, permission denial, power loss and reparse-point races; validate cross-user IPC denial.
- [ ] Validate large histories/queue saturation and IPC pagination limitations.
- [x] Use local Windows validation as requested. GitHub Actions is disabled. Claim Linux/macOS support only after actual platform builds/tests pass.
- [ ] Review upgrade/default-path installer behavior and unsigned application-control behavior on another machine.
- [ ] Build release from a clean committed revision; compare ZIP executables with build outputs and verify every published checksum.
- [ ] Record release decision, tag an appropriate version and publish exact tested artifacts only after required gates pass.

Stable release acceptance remains open; a Windows alpha explicitly retains experimental limits. Local package checks are the release standard. Remaining manual native interaction checks must be disclosed.

Useful commands: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo build --release --workspace --locked`, `cargo audit`, `cargo deny check`, `node scripts/windows-smoke.mjs`, and the packaging commands in INSTALLATION.md. The smoke script terminates only its own spawned fixture/service and isolates state under `.smoke`.
