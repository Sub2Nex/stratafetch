# Phase 2 engineering record

Windows x64, 2026-10-11. Work continues from e4ce03a35e20d95ef06fe1efa7aa7d11c84dd116. This remains an unsigned development preview until release gates pass.

## Established baseline

Exported HEAD using `git archive`, extracted into `.smoke/clean-source`, and built with a separate empty `target/clean-verification` directory and incremental compilation disabled. `cargo test --workspace --offline` passed every target: 21 engine integration tests, four framing tests, one actual Windows IPC roundtrip, binary wrappers and doc tests (26 substantive tests). No test was skipped.

The previous Windows error 4551 has an evidenced cause: Windows Code Integrity Operational events 3033/3077 at 11:32:38 local time identify the browser-host test executable and signing-policy rejection; correlated event 3118 identifies Smart App Control. Policy ID 0283ac0f-fff1-49ae-ada1-8a933130cad6. This is an application-control signing decision, not evidence of antivirus detection. No policy or protection was changed. Subsequent unsigned builds have been allowed; this does not guarantee acceptance on other systems.

A non-elevated sandbox run failed the IPC test by timeout. The clean-source execution with permission for local IPC/HTTP passed. This environment restriction is separate from Code Integrity.

## First verified milestone

- Persisted aggregate bandwidth limit, maximum simultaneous downloads and connections for new transfers.
- GUI settings and CLI `settings --connections 4 --concurrent-jobs 3 --global-bytes-per-second 0` use the same engine commands. Zero rate means unlimited.
- GUI per-job rate and priority controls; pause and wait before changing an existing transfer.
- Aggregate pacing applies to all payload streams, including retries, before disk writes. HTTP probes/headers and kernel socket buffers are outside payload accounting. Quanta are bounded to 16 KiB; no initial token burst. Very high limits may underutilize throughput because pacing uses 20 ms timer steps. Fairness and performance need broader measurements.
- Concurrency admission serialized with pause/cancel; reducing a limit lets existing transfers finish. Resumed files retain their segment layout.
- Fixed simultaneous add race: destination reservation and durable sequence assignment now serialized. Remove/configure/restart checks also retain the control lock during mutation.
- Full workspace suite passed 29 substantive tests after changes. New tests compare actual output bytes, measure aggregate pacing across two jobs, reopen settings, change runtime limits, cancel a 1 B/s throttle, and reserve sixteen filenames concurrently.

## Remaining release gates

Installer, actual native GUI interactions, actual browser extension registration/communication, CI evidence, larger-than-RAM transfers, disk-full fault injection, crash stress and cross-user IPC denial remain to be completed or explicitly retained as limitations. Priority A features after the first four, and most convenience/advanced-network features, are not yet implemented. No production-readiness or comparative acceleration claim is made.
