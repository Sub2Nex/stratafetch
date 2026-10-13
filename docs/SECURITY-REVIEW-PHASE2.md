# Phase 2 security review

This is a source review and targeted local verification, not an independent penetration-test certification.

| Boundary | Findings and evidence |
|---|---|
| URL intake | HTTP/HTTPS and host required; embedded credentials rejected. Native frames deny unknown fields and cap allocations. Query tokens remain private user data and appear in local history/exports. |
| Remote responses | Exact ranges, total sizes, identity encoding, validators and bounded probes enforced. Tests reject changed validators, wrong bounds, truncation, ignored ranges and integer-overflow totals. |
| Publication/files | UUID staging paths, filename sanitization, no-overwrite hard links, prefix hashes and final SHA-256. Concurrent reservation race fixed and stress-tested. Existing user files preserved. Reparse/symlink TOCTOU hardening remains open; hard-link publication requires a supporting filesystem. |
| IPC | No TCP listener; Windows protected owner/SYSTEM DACL, remote clients rejected; Unix socket mode 0600. Protocol bounds and timeouts tested. Handler work now runs off asynchronous I/O workers; write timeouts added and errors reported. Actual different-user denial remains unverified. |
| SSRF | A downloader intentionally permits explicit user-requested local/intranet HTTP URLs. The browser host has an exact extension-origin allowlist and only ping/add operations. There is no unauthenticated remote service, automatic interception or extraction of browser credentials. Network destination policies for managed environments remain future work. |
| Credentials/redirects | No custom authorization/cookie feature or embedded URL credentials; normal TLS verification and redirect limit. Authenticated custom requests require separate redirect-origin policies before implementation. |
| Resource limits | Up to 32 active jobs/16 connections each; stream-to-disk paths, 64 KiB hashing buffers and 16 KiB limited-rate quanta. History import at most 8 MiB/1000 entries, batch at most 256 jobs; imported checkpoints/paths/checksums are not accepted. Large queued-task counts and the 1 MiB IPC reply limit still need pagination/scalable dispatch. |
| History import/export | Explicit destination, safe names and HTTP URLs validated before mutations. Fresh IDs, paused status and no partial state. Existing export files are preserved. I/O failures may leave a paused partial import; no atomic all-or-nothing import promise. |
| Browser/install | Minimal nativeMessaging/contextMenus/activeTab permissions. Registration uses per-user keys and UTF-8 manifests. Registration, origin rejection, actual native frames, IPC, unavailable-service errors and cleanup tested. Installed browser extension flows remain experimental. Uninstaller deletes listed product files and preserves user data. |
| Dependencies | cargo-audit found zero known advisories/warnings using the existing downloaded database (AUDIT-phase2.json). cargo-deny advisories/bans/licenses/sources passed; duplicate-version warnings retained. No new runtime dependency added in Phase 2. |

No verified critical remote/data-corruption vulnerability was found in the reviewed paths. This does not close the remaining filesystem-race, disk-fault, cross-user and saturation verification gates. No telemetry, advertisements, credential collection, security-policy changes or publisher-signature claims were introduced.
