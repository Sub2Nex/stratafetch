# Publishing and local validation

Public source: https://github.com/Sub2Nex/stratafetch.

All validation and packaging run locally. GitHub Actions is disabled and hosted workflows and automated dependency-update configuration are removed by user request. No billing configuration or hosted runner is required. Linux x64 development builds and local native rendering are now exercised; macOS runtime acceptance remains pending. See PLATFORMS.md for exact evidence and limits.

Follow RELEASE-CHECKLIST.md before publishing. Experimental Windows prereleases must clearly disclose unsigned binaries, manual GUI/browser checks, and remaining fault/security hardening. Stable release requirements are stricter. Rebuild packages from a clean committed source revision, compare archive executables to build outputs and publish their exact SHA256SUMS.

```sh
git tag -a VERSION -m 'StrataFetch VERSION'
git push origin VERSION
gh release create VERSION dist/stratafetch-desktop.exe dist/stratafetch-windows-x64.zip dist/SHA256SUMS --prerelease --title 'StrataFetch VERSION' --notes-file docs/RELEASE-NOTES.md
```

These are publication instructions; the Releases page is authoritative about published versions. Hosted checks are neither invoked nor claimed.

`portable.ps1` produces the standalone desktop executable and the full ZIP, with both listed in `SHA256SUMS`. The standalone executable runs the desktop without extraction; the ZIP also includes CLI/browser-host executables, extensions, documentation and licenses. The standalone executable is portable, not an installer.

To add a missing desktop asset to an existing release, first verify that release's ZIP against its published checksum, then extract its `stratafetch-desktop.exe` unchanged. Generate `SHA256SUMS-desktop` for the extracted file and upload the two new assets with `gh release upload VERSION dist/stratafetch-desktop.exe dist/SHA256SUMS-desktop`. Preserve the existing ZIP and checksums. Do not attach binaries rebuilt from later bug fixes to an older source tag; those belong in a new release.

Published Windows alpha: https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-alpha.1. Release source: d8cda2737db90c9776c82f6018b0987c27634040. That tag preserves the original release; current main includes the Phase 3 redesign. To reproduce/verify the published package against its exact source metadata, check out v0.1.0-alpha.1.

Phase 3 source and local native UI evidence are documented in PHASE3.md. The alpha.2 prerelease publishes only the portable ZIP and SHA256SUMS; do not attach an older installer to that tag. Alpha.1 remains historical and immutable. BUILD.txt identifies each portable artifact's exact source commit.
