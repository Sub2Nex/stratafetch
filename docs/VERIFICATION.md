# Verification record

Environment: Windows x64, Rust 1.99.0 stable MSVC, 2026-10-11.

Engine checks: five integration test functions passed serially after sandbox network restrictions were removed. They cover correct segmented bytes, ordinary fallback, unknown-length streaming, inconsistent ranges, HTTP 200 on a range request, truncation, zero-byte responses, collisions, pause/checkpoint/recovery, changed ETag, restart, metadata bounds above 4 GiB, input restrictions, and exclusive process locking. A concurrent rerun and the added integrity changes remain to be verified.

Workspace check passes against eframe 0.36.2. Native binaries, GUI launch, broader integration tests, audits, and packaging are being verified. This file will be updated with final results. No benchmark has been measured. No Linux or macOS build has run locally. GitHub CLI is unauthenticated; no repository or release has been published.
