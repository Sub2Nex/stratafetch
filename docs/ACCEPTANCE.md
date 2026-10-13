# Development preview acceptance

Phase 2: complete workspace tests now pass (32 substantive tests), including the previously blocked executable wrappers. The source is published at https://github.com/Sub2Nex/stratafetch. A real NSIS installer has passed isolated install/uninstall checks. Actual native-host protocol/IPC, origin rejection, service-unavailable errors, registry cleanup and a process-kill/resume transfer passed. See PHASE2.md for current evidence and remaining release blockers. The historical preview table below records the earlier iteration and is not the current release decision.

| Requirement | Evidence/status |
|---|---|
| Native desktop launches | Actual Windows wgpu launch, framebuffer capture, exit 0 |
| Paste URL and use UI controls | Implemented; full manual UI/keyboard walkthrough remains open |
| Correct real file download | 8 MiB fixture, independently checked SHA-256 |
| Real progress/speed/status | Live native framebuffer and engine checkpoint measurements |
| Multiple jobs | Deterministic concurrent-file integration test |
| Pause/resume | Engine tests and native shutdown → standalone CLI resume |
| Restart recovery | SQLite recovery tests, local prefix validation, finalization-boundary recovery |
| Verified ranges/fallback | Exact bytes, unsupported ranges/no validator fallback tests |
| Invalid server responses | Wrong bounds, HTTP 200, truncation, changed validators rejected |
| Open completed files | Implemented OS delegation; manual flow remains open |
| Configure destination | Add dialog/settings and CLI; manual UI review remains open |
| CLI and local control | Actual release CLI transfer, status, resume, checksum; IPC test |
| Persistent history | Actual state survives desktop/CLI process boundaries |
| Native GUI/working engine connection | Actual live transfer screenshot |
| Automated tests | 26 substantive tests pass; final full-workspace run encountered a Windows policy block on an empty test wrapper |
| Primary platform build | Windows x64 MSVC release binaries verified |
| MIT OR Apache-2.0 | Full source license texts; dependency notices retain upstream licenses |
| Reproducible project | Locked workspace, reproducible local test/build/package commands |
| Privacy | No ads, analytics, mandatory accounts, or tracking SDK |

This preview does not meet every stable-release gate. See ROADMAP.md and VERIFICATION.md. No v0.1.0 tag or public release has been created.
