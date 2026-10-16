# Browser integration

The full StrataFetch setup includes Chrome/Edge and Firefox extension sources, a native bridge and guided setup. Store-upload packages and listing text are described in [EXTENSION-PUBLISHING.md](EXTENSION-PUBLISHING.md). Store listings and a signed Firefox XPI remain pending; local developer loading is available now.

## Windows setup

1. Install the full Windows setup package. Leave **Local browser integration** selected to register the bridge for your Windows user. The finish-page launch opens **Settings → Browser integration**.
2. Select **Set up Chrome**, **Set up Edge** or **Set up Firefox**. StrataFetch registers the bridge and opens the browser setup page and the extension folder.
3. Complete the browser's own local-extension steps below. No extension ID needs to be copied and no registration command is needed.

If you upgraded only the desktop executable with the in-app updater, install the full matching setup once to update the native host and extension files too. Setup detects missing or mismatched components. Keep the installation folder in a permanent location.

### Chrome and Edge

On the extensions page, enable **Developer mode**, select **Load unpacked** and choose the **chromium** directory opened by Setup. Both browsers use the same bundled extension. Its fixed public key keeps the local ID stable when the install path changes.

Chrome's ordinary Windows “extension added / action required” prompt is available to store-hosted external extensions. An unsigned local extension uses Load unpacked instead; StrataFetch does not manufacture a browser warning or alter browser policies to force installation.

### Firefox

On **about:debugging → This Firefox**, select **Load Temporary Add-on** and choose **firefox/manifest.json** from the opened directory.

This local unsigned extension must be loaded again after Firefox restarts. Standard Firefox requires Mozilla signing for permanent installation, and no longer supports installation by an external application's sideload registry entry. This package does not disable signature checks or claim a permanent unsigned install.

## Download behavior

The extension popup includes **Handle browser downloads with StrataFetch** (enabled initially; a disabled setting survives extension updates).

For an eligible HTTP/HTTPS download:

1. The extension pauses the browser copy and opens a **Download with StrataFetch** confirmation.
2. **Keep in browser** resumes the original download.
3. **Download** checks whether the URL can be downloaded without browser session data, opens StrataFetch and starts the native transfer.
4. Only after the native transfer receives data, or completes an empty file, does the extension cancel the browser copy. If handoff fails, it resumes the browser download.
5. StrataFetch focuses its existing window or opens it automatically. A second launch uses the same desktop window.

Multiple downloads pause immediately and confirmation prompts open one at a time. Queued items are checked again before their URL is sent to the native host. The confirmation uses the app's own window; no separate Linux message-dialog utility is required. Closing it or pressing Escape keeps the file in the browser.

The extension recovers its own interrupted paused handoffs when its background process/browser starts again, matching both download ID and start time. Failed resume attempts retain their recovery record for another attempt. If the browser cannot resume automatically, the extension badge tells you to use Resume in the browser's download panel.

Private downloads, blob/data URLs, known unsafe downloads, extension packages, webpage exports and downloads started by another extension stay in the browser. Cookies, POST bodies, authentication headers and browsing history are not copied. Login-dependent files may require the browser; size/type/login-page checks reject common session-dependent responses. This is a direct-URL handoff, not a replacement for every browser-specific download mechanism. The cancelled browser history entry is retained.

You can also use **Download with StrataFetch** in a link's context menu or send a direct URL through the extension popup.

## Linux and development use

From an extracted complete package in a permanent directory:

```sh
./stratafetch-browser-host --register-local
./stratafetch-desktop --browser-setup
```

The same setup buttons and local browser-loading steps apply. Native dialogs need a working desktop session and its XDG desktop portal. Linux desktop rendering/download tests do not by themselves verify every portal/browser installation.

The native host retains explicit advanced registration:

```sh
./stratafetch-browser-host --register --chrome-id YOUR_ID --firefox
./stratafetch-browser-host --unregister
```

Windows advanced/manual registration remains available in **packaging/register-browser-host.ps1**; the installer and app use native Windows registry APIs and need no PowerShell execution-policy changes.

## Remove

Disable capture in the extension popup to keep future downloads in the browser. Remove the extension in the browser's extension manager. Uninstalling StrataFetch removes its matching native bridge registration and preserves downloaded files and history. Advanced native bridge removal: **stratafetch-browser-host --unregister-local**.

Registration refuses to overwrite another installation's bridge. Unregister the previous installation before moving the native host. Browser extension approval is always controlled by the browser.

## Platform references

- [Chrome external extension installation](https://developer.chrome.com/docs/extensions/how-to/distribute/install-extensions)
- [Edge extension distribution](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/alternate-distribution-options)
- [Firefox ended installer sideloading](https://blog.mozilla.org/addons/2020/03/10/support-for-extension-sideloading-has-ended/)
- [Firefox signing and self-distribution](https://extensionworkshop.com/documentation/publish/self-distribution/)
