# Security policy and threat model

Please report vulnerabilities privately to the repository owner through GitHub private vulnerability reporting once a repository exists. Do not post credentials or sensitive URLs in public issues. A supported stable release has not yet been declared.

Untrusted inputs include URLs, server headers/bodies, browser requests, and local IPC messages. HTTP/HTTPS only; embedded credentials are rejected. TLS validation remains enabled. Filenames are sanitized, reserved Windows device names are escaped, and filenames are confined to a canonical destination directory. UUID staging filenames and no-overwrite publication avoid accidental file replacement. Existing symlink staging files are rejected. Requests use identity encoding. Range validation, local prefix hashes, bounded streaming, concurrency/connection caps, retry limits, and bounded protocol frames control corruption and resource use.

The local OS account and destination directory owner are trusted. A process running as that same user can access state and downloaded files. Windows IPC uses a protected owner/System DACL and rejects remote clients. Different-user denial testing, malicious directory replacement/reparse-point race hardening, and state-path symlink hardening remain outstanding. Do not use an attacker-writable download directory. Query strings may contain signed URL secrets: state is stored locally, so protect and do not share the database. reqwest errors are stripped of URLs; HTTP bodies are not logged.

Browser integration has no cookie or history collection. Native origins must be registered explicitly. Downloaded executables are untrusted and never automatically launched. An explicit Open File action delegates to the OS.

CI includes cargo-audit and cargo-deny checks. Dependency audit results are recorded in verification notes; configuring a check does not mean it has passed. Before stable release, resolve all applicable audit findings, verify platform access controls, and expand fault-injection testing.
