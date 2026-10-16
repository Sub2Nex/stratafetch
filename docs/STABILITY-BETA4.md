# Beta.4 stability audit — 2026-10-16

No particular user crash was reported. These fixes came from code inspection and local reproductions.

## Confirmed defects and fixes

| Area | Before | After |
| --- | --- | --- |
| Probe recovery | A response disconnected after valid range headers failed immediately. | Body interruptions retry within the configured budget; exhaustion fails without publishing a file. |
| Low bandwidth limits | A whole HTTP chunk could delay progress for many seconds, then commit an unpaced burst when paused. | Small paced writes produce regular durable checkpoints; both individual and global 1 KiB/s tests pass and resume to exact bytes. |
| Windows IPC | A temporarily busy named pipe caused an immediate connection error. | Retry connection admission within the existing timeout; a native contention test verifies exactly one add. |
| Desktop exit | Dropping the command sender drained the queued requests before shutdown. | A prioritized stop signal discards pending work and pauses the owned engine. Already executing operations finish first. |
| Transfer bookkeeping | Restarted downloads retained their old speed baseline; removed jobs retained session data. | Reset the baseline when byte counts regress and prune removed records from session/preferences tracking. |
| Windows packaging | Version checks rejected ordinary CRLF checkouts and metadata always said MSVC. | Accept CRLF and record the selected GNU/MSVC target. |

Both interrupted-probe and low-limit tests failed against the original engine and passed after the patch. The native Windows CLI regression harness also now accepts `STRATAFETCH_TEST_CLI`, because cross-built tests embed a build-host path that does not exist on Windows.

## Verification completed

- **88 Rust tests pass on Linux**, including the optimized Ubuntu 24.04 release suite.
- **82 Rust tests pass natively on Windows**, including Windows-specific tests. Every test process is awaited and its successful result is checked.
- Strict workspace Clippy passes with warnings denied; formatting passes. The final CLI harness adjustment is checked separately.
- All **18 browser-script tests** pass.
- Actual Windows and extracted Linux release executables complete a CLI/service/native-host ranged download and SHA-256 verification using isolated state and loopback HTTP.
- Actual Windows and extracted Linux desktops render settings and exit cleanly.
- An isolated native Windows beta.3 → beta.4 updater-helper cycle waits for its parent, preserves the exact previous executable, installs the exact new executable, restarts and renders the desktop.
- The NSIS test installer passes native install/reinstall/uninstall with all three executable hashes and version checked. It uses separate app registration/shortcuts, preserves an unrelated sentinel and leaves the existing installation outside the test.
- Windows ZIP contents, source revision, executable hashes, embedded Common Controls v6 manifests and artifact checksums are verified. The extracted Linux binary requires at most GLIBC_2.39.

Application binaries were built from `7e56e662f5caa27230e0b86ba9df0202529081cb`; subsequent release commits contain test-harness/documentation changes. Each package records the application build revision in `BUILD.txt`. All builds ran locally in WSL, without CI or Actions.

Local evidence is under `D:\stratafetch-main\.smoke\beta4`: native test logs/results, settings captures, updater restart capture and smoke logs. WSL logs include `audit-fixed.log`, `audit-clippy.log`, `linux-beta4-tests.log`, `browser-beta4.log` and `linux-beta4-archive-smoke.log` in the build user's home.

This is still an unsigned beta. These checks do not establish exhaustive reliability across machines, installed browser integrations, physical display/accessibility transitions, macOS or ARM64.
