# Browser extension publishing

Create the upload files locally with `python packaging/package-extensions.py --output dist/browser-upload`. Both ZIPs contain Manifest V3 source, all code and icons. No remote code, bundling step, CI or Actions are used.

## Chrome

Upload **stratafetch-chrome-webstore.zip** as a new draft in the [developer dashboard](https://chrome.google.com/webstore/devconsole). This archive omits the local development key. The first upload assigns the store ID and public key; upload alone does not publish it.

Before distributing the store version, set `extensions/local-ids.json` and the key in `extensions/chromium/manifest.json` to the draft's assigned ID/public key, rebuild the full installer, and test native messaging with that origin. Never publish an extension with an incompatible native bridge. The same Chrome listing can be installed in Edge after user approval; a separate Edge Add-ons listing assigns a different ID which also needs registration.

## Firefox

Upload **stratafetch-firefox-amo.zip** in the [Mozilla developer hub](https://addons.mozilla.org/developers/). Choose a listed extension or self-distribution to obtain a signed XPI. Keep the manifest ID **stratafetch@local** for updates and compatibility with the native bridge. The source ZIP is not a signed permanent-install package.

The manifest requires Firefox 142+ and declares `browsingActivity`, `websiteActivity`, and `websiteContent`: download URLs, the download action and filename/size/type metadata sent to the local app. This is required functionality disclosed during installation. There is no developer analytics server. Do not declare `none`: native messaging sends data outside the browser even when processing stays on the computer.

## Listing text

**Name:** StrataFetch

**Summary:** Open eligible browser downloads in StrataFetch with confirmation and automatic browser fallback.

**Description:**

StrataFetch connects browser downloads to the free, open-source StrataFetch desktop download manager. When an eligible HTTP or HTTPS download starts, the extension pauses the browser copy and asks whether to download with StrataFetch. Choose Download to open the app and start the transfer, or Keep in browser to continue normally. The browser copy is cancelled only after StrataFetch acknowledges receiving file data. Failed handoffs resume the browser copy.

Automatic capture is enabled initially and can be switched off in the popup. You can also send a direct link from its context menu or the popup. Install the matching full desktop package and register its native bridge in Settings → Browser integration first. No account is required.

Private downloads, browser extension packages, page exports, blob/data URLs, known unsafe downloads and downloads initiated by other extensions remain in the browser. Cookies, authentication headers and POST bodies are not copied. Files requiring a browser session may need to finish in the browser. Video extraction and protected streaming are not supported.

Download information is passed to the local app and the chosen download server. There is no developer telemetry or advertising. Temporary local download IDs/start times/native handoff IDs support interrupted-handoff recovery. Files and download history remain local.

**Homepage/support:** https://github.com/Sub2Nex/stratafetch

**Privacy policy:** https://github.com/Sub2Nex/stratafetch/blob/main/PRIVACY.md

**Category:** Productivity / download management. **Price:** Free.

## Permission explanations

- `downloads`: observe new downloads, pause for confirmation, cancel after acknowledgment and resume on fallback. Only owned download IDs are changed.
- `nativeMessaging`: connect to the registered local StrataFetch app; pass download URL/name/size/type and receive status.
- `contextMenus`: offer Download with StrataFetch on a link.
- `activeTab`: read the current tab URL when the user presses Use current page URL.
- `storage`: save capture preference and temporary recovery IDs/start times/native handoff IDs.

Complete owner certifications truthfully against this implementation. The extension handles download URLs/metadata locally; it does not collect general browsing history, page contents, keystrokes, cookies or credentials, and sends no developer telemetry. Do not claim that no data crosses the browser boundary: native messaging passes it to the local app.

## Reviewer checks

Install the matching desktop package and register the correct extension origin. No credentials are needed. With capture enabled, start a public direct-file download and choose Keep in browser; it resumes without a native transfer. Repeat and choose Download; StrataFetch opens, receives file data and the browser copy is cancelled. Turn capture off; a new download stays in the browser. With the companion unavailable or an anonymous URL returning a login page, the original browser copy must resume. Multiple downloads should prompt one at a time.

Use actual installed-extension screenshots for store media. Signing, review, public listing URLs and Chrome's assigned origin remain pending until the submissions are complete.

## Official references

- [Chrome upload and review](https://developer.chrome.com/docs/webstore/publish)
- [Chrome key and extension ID](https://developer.chrome.com/docs/extensions/reference/manifest/key)
- [Firefox signing and distribution](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/)
- [Firefox built-in data consent](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
