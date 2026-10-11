# Architecture

`strata-core` is independent of the GUI. `Manager` owns SQLite state, jobs, a bounded concurrent-job semaphore, control channels, and an HTTP client. `network` performs streaming probe/range/single-stream transfers; `storage` maintains integrity-checked JSON job snapshots in SQLite. Separate crates for these modules are deferred until their APIs stabilize.

`strata-app` contains the native desktop, CLI, native messaging host, typed commands, and OS-local IPC. The desktop sends bounded commands to a Tokio runtime on another thread and renders snapshots. Network, disk, and database work does not execute on its rendering thread. Virtualized history rows avoid drawing the entire history.

## Transfer states

New jobs are Paused. Start transitions to Queued, then Downloading once capacity and priority permit. Success transitions to Completed. Errors transition to Failed. Pause/Cancel cooperatively stop queued or active work, preserving verified checkpoints. Resume queues remaining ranges. Restart removes staging data and clears validators. Completed jobs cannot be restarted or cancelled; add another job instead. Active jobs cannot be removed or reconfigured. Recovery changes Queued/Downloading to Paused.

## Byte safety

A GET `bytes=0-0` probe determines actual range support. Safe segmentation additionally requires a strong ETag or Last-Modified validator. Ranges are nonoverlapping and have independent file handles. Every response must be HTTP 206 with the exact expected bounds, total, validator, and identity encoding. Too many bytes, truncation, or changed validators prevent publication. Resumed requests carry If-Range.

Durable file synchronization precedes each SQLite checkpoint. SHA-256 prefix digests validate saved partial segments before resume. Uncheckpointed trailing bytes are rewritten. SHA-256 is computed by streaming the final staging file. A same-directory hard link publishes without replacing an existing destination; SQLite records completion before staging cleanup. Filesystems without hard-link support fail clearly and keep staging data. Recovery after a crash between publication and database completion needs manual inspection; the engine will not overwrite the published file.

## Coordination

An exclusive state lock prevents competing writers. IPC uses named pipes on Windows with remote clients rejected, or Unix sockets with mode 0600. The protocol is versioned and limited to 1 MiB, with timeouts and 16 concurrent handlers. Native messaging is limited to 64 KiB and checks registered extension origins. IPC has no TCP listener. Windows relies on the pipe creator's default DACL; explicit per-user DACL hardening is a release follow-up.

## Extension points

Protocol adapters, dynamic segment splitting, scheduled queues, categories, structured authenticated requests, HLS/DASH parsers and muxing, and explicit remote control can be added behind core interfaces. These are not implemented in this preview.
