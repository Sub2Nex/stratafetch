# Beta.5 browser verification

All builds and checks are local; no CI or Actions were used.

- 99 JavaScript extension tests pass for Chromium and Firefox sources: automatic capture, acknowledgment before cancellation, decline/failure fallback, queued prompts, late cancellation, post-pause privacy/type checks, closed popup relay, native port lifetime/send failure, accurate readiness and restart recovery.
- 94 Linux Rust tests pass, including real HTTP/IPC file bytes after a confirmed transfer, rejection before a job is created, confirmation-button pointer input, explicit approval exit codes, desktop locking and ownership-preserving registration.
- Strict Clippy passes on Linux and the Windows GNU target, including all targets/tests.
- The actual Linux native host accepts native-message frames, reports bridge readiness while the app is closed, rejects a blob URL, launches the GUI confirmation and declines with zero requests to the loopback download server. The actual dialog framebuffer was captured and inspected.
- Isolated Windows PowerShell registration tests pass against real HKCU test keys: distinct Chrome/Edge origins, ownership refusal, repeat registration and cleanup preserving manual origins/history. Production browser registrations were not changed by tests.
- Mozilla web-ext validation reports 0 errors, 0 notices and 0 warnings. Firefox 142+ supports the manifest data-consent declaration. Source archives remain unsigned; lint is not signing or approval.

Windows application control rejected the new native host with Code Integrity event 3077 under policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`. Native Windows beta.5 handoff and installer execution remain unverified. No application-control, antivirus, browser signing or extension policy was disabled or bypassed. Earlier beta.4 Windows results do not establish beta.5 execution.

No complete installed-browser capture workflow is claimed from these tests. Extension behavior uses API fixtures; the native host/HTTP/IPC and dialog rendering are exercised separately. Protected URLs, cookies, POST bodies, private downloads and browser-generated streams stay in the browser as described in BROWSER.md. Chrome store origin assignment, reviews and Firefox signing remain pending.

Local evidence is in `.smoke/browser-beta5/`. WSL home logs: `browser-beta5-tests.log`, `browser-beta5-clippy.log`, `browser-beta5-windows-clippy.log`, `browser-beta5-native-smoke.log`, `browser-beta5-firefox-lint.log`. Machine-specific logs are excluded from source packages.
