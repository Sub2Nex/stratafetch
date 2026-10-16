# StrataFetch v0.1.0-beta.5 — Automatic browser download prompts

**[Download the full Windows setup installer](https://github.com/Sub2Nex/stratafetch/releases/download/v0.1.0-beta.5/stratafetch-windows-x64-setup.exe)**

## Browser downloads

- Eligible HTTP/HTTPS downloads pause for a **Download with StrataFetch** confirmation. Acceptance opens/focuses the app, starts the transfer and cancels the browser copy only after the native transfer receives data.
- Decline or failed handoff resumes the original browser download. Interrupted owned pauses are recovered when the extension/browser starts again.
- The extension popup can disable automatic capture. Private downloads, extension packages, webpage exports, blob/data URLs and known unsafe downloads stay in the browser. Cookies and browser session headers are not copied; protected/POST downloads may require the browser.
- A second desktop launch focuses the existing window instead of opening another one.
- Simultaneous downloads prompt one at a time. Queued downloads are rechecked, failed resume attempts retain recovery records, and bridge readiness no longer requires the app to be running.
- The focused confirmation uses StrataFetch's own UI on Windows and Linux. Closing it or Escape declines; ordinary/older desktop exit codes cannot approve downloads.

## Local setup

The installer includes an optional **Local browser integration** component. It registers the native bridge using Windows APIs, without PowerShell policy changes, and opens Settings → Browser integration on its finish-page launch. Setup buttons open the browser setup page and the local source folder; no extension ID or registration command needs copying.

**Chrome / Edge:** enable Developer mode on the extensions page, choose **Load unpacked** and select the bundled **chromium** directory.

**Firefox 142+:** choose **Load Temporary Add-on** on about:debugging → This Firefox and select **firefox/manifest.json**. Unsigned local extensions must be reloaded after Firefox restarts.

Store-upload packages, icons, Firefox data declarations and listing/reviewer text are included. Store review/signing and public listings remain pending. Chrome's assigned store origin must be registered in a matching installer before store distribution; registration accounts alone do not provide that ID. Firefox permanent installation requires a Mozilla-signed package. [Setup instructions](https://github.com/Sub2Nex/stratafetch/blob/main/docs/BROWSER.md) · [Publishing files and text](https://github.com/Sub2Nex/stratafetch/blob/main/docs/EXTENSION-PUBLISHING.md).

## Downloads

- **stratafetch-windows-x64-setup.exe** — recommended; desktop, CLI, native host, local extensions, optional shortcuts and uninstall entry.
- **stratafetch-linux-x64.tar.gz** — full Linux x64 package built on Ubuntu 24.04, requiring GLIBC_2.39, X11/Wayland/Vulkan and a desktop portal for native dialogs.
- **stratafetch-chromium-local.zip** and **stratafetch-firefox-local-source.zip** — unpacked extension sources, with manifest.json at the archive root. The Firefox archive is not a signed XPI.
- **stratafetch-chrome-webstore.zip** and **stratafetch-firefox-amo.zip** — draft upload packages; these are not approved store listings or signed installation packages.
- **stratafetch-desktop.exe**, **stratafetch-windows-x64.zip**, **SHA256SUMS** — updater executable, full Windows archive and all artifact hashes.

Install the full matching setup/package once for this browser feature. The existing in-app updater replaces only the desktop executable. Setup detects missing/older companion files and directs you to the full package.

## Verification and scope

99 extension tests and 94 Linux Rust tests pass, including actual confirmation-button input and real HTTP/IPC bytes after acceptance. Strict Clippy passes for Linux and the Windows target. The actual Linux native host renders the confirmation framebuffer, reports readiness with the app closed, rejects unsupported URLs and declines with zero download-server requests. Isolated real Windows HKCU registration tests pass for Chrome/Edge origins, ownership refusal, repeated setup and cleanup preserving unrelated origins/history. Mozilla's web-ext linter reports zero errors, notices or warnings.

All builds run locally without CI or Actions. Windows binaries and Firefox sources remain unsigned. This machine's Windows application-control policy blocked the new native-host executable, so beta.5 native Windows handoff/installer execution is unverified; no security policy was changed. Linux native checks and extension regression fixtures do not establish a complete installed Chrome/Firefox/Edge or protected-site workflow.
