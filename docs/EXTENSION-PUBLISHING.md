# Browser extension publishing

## Complete per-store submission kit

Run `python scripts/package-store-listings.py --output dist/browser-upload`, then `python packaging/package-extensions.py --output dist/browser-upload` to create the archives, copy-and-paste fields, shared icon, promotional artwork and checksums together. Each of `chrome/`, `edge/` and `firefox/` contains START-HERE.md, name, short/full description, release notes, publisher/support metadata, permission justifications, privacy answers and reviewer notes. The new universal artwork lives in assets/branding; scripts/export-branding.py reproduces its exports using Pillow.

The initial ZIP can create a store draft before its assigned Chrome/Edge ID is known. Record the actual ID and rebuild the native companion before review; do not advertise a working store-origin bridge until that installation is tested. Store signing and real screenshots remain separate submission steps.


Create the upload files locally with `python packaging/package-extensions.py --output dist/browser-upload`. Chrome, Edge and Firefox ZIPs contain Manifest V3 source, all code and icons, with manifest.json at each archive root. The output also includes unpacked local sources, an unsigned temporary-development XPI, this guide, privacy/release text and SHA256SUMS. No private keys, remote code, caches, CI or Actions are included.

## Chrome

Upload **stratafetch-chrome-webstore.zip** as a new draft in the [developer dashboard](https://chrome.google.com/webstore/devconsole). This archive omits the local development key. The first upload assigns the store ID and public key; upload alone does not publish it.

Do not change `extensions/local-ids.json` or the development manifest key. Record assigned store IDs with the helper below. Store identities are kept separately in `extensions/store-ids.json`, initially null. The Chrome listing can also be installed in Edge after approval, but an Edge Add-ons listing assigns a separate ID.

## Microsoft Edge

Upload **stratafetch-edge-addons.zip** in the [Edge developer dashboard](https://partner.microsoft.com/dashboard/microsoftedge/overview) as a new extension draft. The ZIP contains Chromium-compatible source and omits the development key. Record Edge's actual 32-letter extension ID separately from Chrome's ID; a submission/product identifier may be different.

## Record assigned identities and rebuild

Use the actual IDs from each store. Uppercase words below are placeholders, not valid IDs:

```sh
python packaging/configure-store-id.py chrome --id ACTUAL_CHROME_EXTENSION_ID --public-key-file chrome-public-key.txt
python packaging/configure-store-id.py edge --id ACTUAL_EDGE_EXTENSION_ID
python packaging/package-extensions.py --output dist/browser-upload
cargo build --release --workspace --bins --locked
```

The public-key option is optional until the store exposes its key. Supply only a public key; the helper rejects private keys and keys that do not derive the supplied ID. Optional `--listing-url` accepts HTTPS on that browser's official store domain. The helper never changes the development ID/key.

Rebuild the full Windows installer or Linux package from the same committed configuration. Install it, then use Settings → Browser integration or `stratafetch-browser-host --register-local`. Native manifests allow the stable local origin and explicitly configured Chrome/Edge store origins together. Firefox retains its stable manifest ID. Unregister preserves unrelated manually registered origins.

When a store public key is available, extract its generated store ZIP into an unpacked directory in a separate test profile and verify that its displayed ID matches the assigned ID before native messaging tests. Otherwise use the store's supported tester distribution when available. Verify both confirmation choices, then repeat with the original local development directory to check that both identities work. Do not distribute an extension with an incompatible bridge.

No production ID is needed for the initial upload ZIPs. Store submissions remain manual owner actions.

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
