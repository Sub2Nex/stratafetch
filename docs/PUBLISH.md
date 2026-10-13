# Publishing and local validation

Public source: https://github.com/Sub2Nex/stratafetch.

All validation and packaging run locally. GitHub Actions is disabled and hosted workflows and automated dependency-update configuration are removed by user request. No billing configuration or hosted runner is required. Linux/macOS support remains unverified until actual builds/tests on those platforms pass.

Follow RELEASE-CHECKLIST.md before publishing. Experimental Windows prereleases must clearly disclose unsigned binaries, manual GUI/browser checks, and remaining fault/security hardening. Stable release requirements are stricter. Rebuild packages from a clean committed source revision, compare archive executables to build outputs and publish their exact SHA256SUMS.

```sh
git tag -a VERSION -m 'StrataFetch VERSION'
git push origin VERSION
gh release create VERSION dist/stratafetch-windows-x64.zip dist/stratafetch-windows-x64-setup.exe dist/SHA256SUMS --prerelease --title 'StrataFetch VERSION' --notes-file docs/RELEASE-NOTES.md
```

These are publication instructions; the Releases page is authoritative about published versions. Hosted checks are neither invoked nor claimed.
