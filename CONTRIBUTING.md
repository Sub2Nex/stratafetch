# Contributing

Use stable Rust, format code, run Clippy and the test suite, and add deterministic regression tests for behavioral changes. Keep networking/storage out of the GUI thread. Use bounded streams and validate all range metadata before writing. Never weaken TLS validation or overwrite user files to make a test pass.

Submit small, reviewable changes with the concrete problem, resulting behavior, and validation evidence. Describe limitations accurately. Security reports belong in the private channel described in SECURITY.md. All contributions are MIT OR Apache-2.0.
