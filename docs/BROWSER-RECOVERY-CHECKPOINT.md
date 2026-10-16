# Browser recovery checkpoint — 2026-10-16

## Source and publication

The real Git checkout is `D:\stratafetch-main\.publish`, branch `main`, remote `https://github.com/Sub2Nex/stratafetch.git`. `D:\stratafetch-main` contains a synchronized source copy, not a second Git checkout. Preserve both; do not reset, clean or force-push.

Application source/tag: **072beecfce86db54538bf998d9a0912422f2ae81**, **v0.1.0-beta.5**. Main includes this subsequent documentation-only checkpoint. The earlier commits 8143f51 and 52e999e contain the browser implementation and separate store identities.

Published [beta.5](https://github.com/Sub2Nex/stratafetch/releases/tag/v0.1.0-beta.5) as GitHub **Latest**, release ID **415193623**. All 11 uploaded asset sizes and SHA-256 digests match the finished local files, including SHA256SUMS itself. Publication was previously authorized; no browser-store submissions were made.

## Finished files

Canonical extension directory: **D:\stratafetch-main\dist\browser-upload-beta.5**.

- `stratafetch-chrome-webstore.zip`, `stratafetch-edge-addons.zip`, `stratafetch-firefox-amo.zip`: initial draft submission packages with manifest.json at root, all code/icons and no development Chromium key.
- `stratafetch-chromium-local.zip`, `stratafetch-firefox-local-source.zip`: local development sources.
- `stratafetch-firefox-unsigned-dev.xpi`: unsigned temporary-development archive; not a permanently installable standard-Firefox package.
- `unpacked\chromium`, `unpacked\firefox`: inspected local extension directories.
- `SUBMISSION-GUIDE.md`, `PRIVACY.md`, `RELEASE-NOTES.md`, `SHA256SUMS`: owner instructions, disclosures and recursive checksums.

Desktop release directory: **D:\stratafetch-main\dist\beta.5**. Contains Windows setup, desktop executable, full Windows ZIP, Linux x64 archive, all six extension archives above and shared SHA256SUMS. The older `.publish\dist\browser-upload` staging alias was also refreshed; prefer the canonical versioned directory.

Key desktop hashes:

| File | SHA-256 |
| --- | --- |
| stratafetch-windows-x64-setup.exe | 94439f939f3717bb01be1ec40dac3df11a3ebbd51480d4d7e092d036b0815865 |
| stratafetch-windows-x64.zip | 13461d20cf65f853acefe8ccd7d57dd641a13f21e125833421ba2177eb06120e |
| stratafetch-desktop.exe | 9936de2233ca349d90bac62587b03496c0115da9c90eefef157aba9e4401e9db |
| stratafetch-linux-x64.tar.gz | 7a421878ce2fc38dfee65298bad3f9748b812b40f3157ec6265daa7f111a0e8e |

## Implementation and validation

Eligible browser downloads pause for one queued StrataFetch confirmation at a time. Decline/close/failure resumes the original. Native data acknowledgment precedes browser cancellation. Recovery matches owned IDs/start times and retains failed resumes. Popup controls, context-menu sends, direct URLs, readiness and domain exclusions are implemented. Private/generated/unsafe downloads, browser extensions, credential-bearing URLs and unsupported protocols remain in the browser; cookies/session headers are not exported.

Local Chromium identity remains **eakfgnhnogmekemikidnkfajbnfkjgph**. Firefox manifest identity remains **stratafetch@local**. `extensions/store-ids.json` has **null Chrome/Edge production entries**; no fictitious production identities were generated. `packaging/configure-store-id.py` records actual assigned IDs/public keys separately without changing local IDs. Native manifests can allow local and explicitly configured store origins together.

- **117** browser JavaScript tests pass for Chromium/Firefox API fixtures; Edge uses the same Chromium source.
- **97** optimized Linux Rust tests pass on Ubuntu 24.04. Formatting and strict Clippy pass on Linux and the Windows target.
- **91** unique native Windows Rust tests pass. After the final theme fix, the affected library/desktop suites were rerun: **39** passed. The remaining unchanged suites retain their successful full-run evidence.
- **4** packaging/identity tests pass: actual initial archives and checksums, deterministic ZIPs, preservation of local identities, and rejection of invalid/private/mismatched keys.
- Mozilla web-ext validation of the **finished unpacked Firefox artifact** reports **0 errors, 0 notices, 0 warnings**. Firefox minimum is 142.
- Extracted Linux packages pass a real CLI/service/native-host ranged download with exact bytes/checksum. Native host framing, offline readiness, unsupported URL rejection and rendered confirmation/zero-request decline pass on Linux and Windows.
- Accepted Windows native handoff produced the exact 131072 fixture bytes, SHA-256 `feb1e4409d009e0ec502eaabe321f86b5197a881e9b765252ec8a75d6957596d`. Both confirmation choices are independently verified with real egui pointer-event tests on Windows.
- The final Windows framebuffer was inspected after fixing light-system-theme contrast. Common Controls v6 manifests are verified in all three executables.
- Actual isolated Windows **install/reinstall/uninstall** passes with matching executable hashes, CLI version, isolated registration/shortcuts removed and an unrelated sentinel file preserved. TEST_BUILD skips production browser registration; real native registry ownership/origin tests cover it separately.

An earlier Windows application-control rejection was superseded by normal execution of updated binaries. No Windows, antivirus, browser signing or extension policy was changed. In-app update/rollback code is preserved; desktop-only updates visibly open browser setup when older package metadata indicates stale companions. Install the full matching setup once for this browser feature.

## Remaining external steps and limits

The owner has registered developer accounts but has **not uploaded to Chrome, Edge or Mozilla**. Upload the three store ZIPs manually, obtain Chrome/Edge extension IDs, record them with the helper in the real checkout, rebuild compatible desktop packages and test those identities. Keep the development ID/key unchanged. Firefox requires Mozilla's official signing process before permanent installation; no signed XPI is claimed.

No complete installed Chrome/Edge/Firefox capture workflow is claimed. Browser interception uses API fixtures; native HTTP/IPC/GUI checks are separate. Two manual OS-click harness attempts could not establish the intended choice because the UI tool lost foreground-process verification; they are inconclusive and **not counted as passes**. Cookies, POST bodies, protected/session-dependent downloads and video extraction remain unsupported by this direct-URL bridge.

## Build environment and evidence

All work is local; no CI/Actions or paid infrastructure. WSL **Ubuntu**, user `strata-build`, builds Windows GNU binaries with target cache `/home/strata-build/target-windows`. **Ubuntu-24.04** builds Linux with GLIBC_2.39, using `/home/strata-build/stratafetch/target`. Exact release checkouts are `/home/strata-build/stratafetch-beta5-release` in each distribution. NSIS builds both production and isolated test installers locally.

Machine evidence: `.publish\.smoke\browser-beta5` and `D:\stratafetch-main\.smoke\browser-beta5`. Important files: `browser-tests.log`, `installer-roundtrip.log`, `github-draft-assets.json`, Windows `native-results.json`, `windows-confirmation.png`, and `windows-tests\combined-results.json`. Keep the inconclusive manual-click logs distinct from passing native smoke checks.

WSL home logs: `browser-beta5-tests.log`, `browser-beta5-clippy.log`, `browser-beta5-windows-clippy.log`, `browser-beta5-linux-release-tests.log`, `browser-beta5-extracted-package-smoke.log`, `browser-beta5-release-native-smoke.log`, `browser-beta5-firefox-final-package-lint.log`, `browser-beta5-installer-build.log`. Read [EXTENSION-PUBLISHING.md](EXTENSION-PUBLISHING.md) for descriptions, permissions, privacy and exact post-ID steps.
