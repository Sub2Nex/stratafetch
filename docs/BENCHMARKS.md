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
