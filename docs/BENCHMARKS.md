# Local fixture timings

Measured 2026-10-11 on Windows x64, stable Rust 1.99 MSVC, optimized release build. The same machine served and downloaded an 8 MiB deterministic fixture over loopback HTTP. Each run used fresh SQLite state and a fresh destination. Timings include probe, durable writes/checkpoints, SHA-256, publication, and completion polling. All ten hashes matched the independently calculated fixture digest.

| Connections | Five completion times (ms) | Median (ms) |
|---|---|---|
| 1 | 69.671, 61.951, 78.958, 75.810, 76.190 | 75.810 |
| 4 | 71.772, 59.752, 60.855, 59.643, 73.784 | 60.855 |

These small loopback trials primarily measure local overhead and cached filesystem behavior. They do not establish WAN throughput, browser/IDM comparisons, CPU use, memory use, or performance on a slow disk. There is no general acceleration claim.

To reproduce, in separate terminals:

```sh
cargo run --release -p strata-core --example fixture_server
cargo run --release -p strata-core --example benchmark
```

The benchmark expects the supplied fixture pattern and refuses a different checksum. Raw data is in benchmark.csv. Longer files, controlled latency/loss, cold caches, CPU/memory sampling, and resume overhead remain future work.

## Phase 2 streaming fixture

The updated fixture streams 64 KiB chunks rather than allocating a whole response. Timings cannot be compared directly with the earlier fixture. Five single-connection times were 127.301, 135.530, 81.207, 120.623, 132.853 ms (median 127.301); four-connection times were 120.896, 90.803, 99.661, 172.966, 117.304 ms (median 117.304). All ten 8 MiB hashes matched. Raw data: benchmark-phase2.csv. This small, variable loopback result does not justify adaptive segmentation or a comparative acceleration claim.

Large streamed-file evidence is in large-transfer-phase2.json and large-transfer-4g-phase2.json. The 4 GiB file exceeded free physical memory at start; the CLI sampled about 18.8 MiB peak working set and completed in 81.801 seconds. The 1 GiB file completed in 11.234 seconds with about 18.7 MiB sampled peak. Independent expected-pattern hashing and reread file hashing verified both results. These are local workload measurements; polling can miss transient peaks and working set is not total reserved memory. No universal RAM ceiling or WAN throughput claim follows.
