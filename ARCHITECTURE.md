# Architecture

`strata-core` is independent of the GUI. `Manager` owns SQLite state, jobs, serialized queue admission, control channels, persisted engine settings, aggregate payload pacing and an HTTP client. `network` performs streaming probe/range/single-stream transfers; `storage` maintains integrity-checked JSON job snapshots and settings in SQLite; `history` validates portable recipes. Separate crates for these modules are deferred until their APIs stabilize.

`strata-app` contains the native desktop, CLI, native messaging host, typed commands, and OS-local IPC. The desktop sends bounded commands to a Tokio runtime on another thread and renders snapshots. Network, disk, and database work does not execute on its rendering thread. Virtualized history rows avoid drawing the entire history.

## Transfer states

New jobs are Paused. Start transitions to Queued, then Downloading once capacity and priority permit. Success transitions to Completed. Errors transition to Failed. Pause/Cancel cooperatively stop queued or active work, preserving verified checkpoints. Resume queues remaining ranges. Restart removes staging data and clears validators. Completed jobs cannot be restarted or cancelled; add another job instead. Active jobs cannot be removed or reconfigured. Recovery changes Queued/Downloading to Paused.

## Byte safety

A GET `bytes=0-0` probe determines actual range support. Safe segmentation additionally requires a strong ETag or Last-Modified validator. Ranges are nonoverlapping and have independent file handles. Every response must be HTTP 206 with the exact expected bounds, total, validator, and identity encoding. Too many bytes, truncation, or changed validators prevent publication. Resumed requests carry If-Range.

Durable file synchronization precedes each SQLite checkpoint. SHA-256 prefix digests validate saved partial segments before resume. Uncheckpointed trailing bytes are rewritten. SHA-256 is computed by streaming the final staging file. A same-directory hard link publishes without replacing an existing destination; SQLite records completion before staging cleanup. Filesystems without hard-link support fail clearly and keep staging data. Recovery checks file identity and prefix checksums when a crash occurs between publication and database completion, and records completion only after validation. This boundary has a regression test.

## Coordination

An exclusive state lock prevents competing writers. IPC uses named pipes on Windows with remote clients rejected and a protected owner/System DACL, or Unix sockets with mode 0600. The protocol is versioned and limited to 1 MiB, with timeouts and 16 concurrent handlers. Native messaging is limited to 64 KiB and checks registered extension origins. IPC has no TCP listener. Windows owner access passed the initial roundtrip test; a different-user denial test remains outstanding. A desktop opened while an engine already runs attaches through IPC.

## Extension points

Phase 2 adds categories, optional extension folders, per-job UTC start times, runtime concurrency/connection settings, global throttling, bounded batch controls and history recipes. The queue skips future jobs and admits due jobs by priority/FIFO. Settings survive engine restarts. Schedules require a running engine and interrupted queued work reopens paused. Imports recreate paused jobs with fresh IDs; exported absolute paths and checkpoint state are never restored. Active partial downloads keep their segment layout when defaults change.

Protocol adapters, dynamic segment splitting, structured authenticated requests, HLS/DASH parsers and muxing, and explicit remote control can be added behind core interfaces. These are not implemented in this preview. IPC history pagination and replacing one polling task per queued job with a scalable dispatcher remain important before very large histories/queues are supported confidently.

## Desktop separation

Phase 3 keeps desktop.rs as the launcher. UI modules separate preferences/selection, original icons/theme, toolbar/tree/table and dialogs. The asynchronous worker isolates blocking engine commands and keeps one latest snapshot. Rendering is virtualized; IDs remain engine IDs. Metadata is an independent header-only helper. AddPaused and batch Remove are additive commands; existing Add still starts transfers. No schema/checkpoint change is introduced.
