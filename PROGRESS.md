# StrataFetch engineering log

2026-10-11: Empty workspace inspected. Rust 1.99 MSVC available; GitHub CLI is unauthenticated. Creating Rust 2024 workspace. Engine correctness precedes desktop work. Initial architecture consolidates networking/storage/scheduling in strata-core and CLI/desktop in strata-app; separate modules will keep these boundaries explicit.

Release gate: do not tag 0.1.0 until engine tests, Windows binaries, and native launch have been verified. Secondary platforms require CI verification.
