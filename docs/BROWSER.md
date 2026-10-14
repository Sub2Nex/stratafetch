# Experimental browser integration

1. Build `stratafetch-browser-host` and keep the executable in a permanent location.
2. In Chrome/Edge enable developer mode on the extensions page, then Load unpacked `extensions/chromium`. Copy its extension ID.
3. Run Windows PowerShell or PowerShell 7: `./packaging/register-browser-host.ps1 -BinaryPath ./target/release/stratafetch-browser-host.exe -ChromiumExtensionId YOUR_EXTENSION_ID`. No administrator access is needed. The script writes UTF-8 without BOM per-user Chrome/Edge/Firefox native-host manifests and registry entries. Review it before running.
   If Edge shows a different ID, also pass `-EdgeExtensionId EDGE_EXTENSION_ID`; both exact Chromium origins are registered. IDs are not inferred from browser history.
   For a portable archive, use `-BinaryPath ./stratafetch-browser-host.exe` instead. The registration must point to its permanent installed location.
4. Start StrataFetch desktop or `stratafetch serve`. Open the popup; its status should say Connected. Send a direct link or use the link context menu.
5. Firefox: load `extensions/firefox/manifest.json` as a temporary extension through about:debugging. Permanent distribution requires Mozilla signing; this preview is not signed.

Unregister with `./packaging/register-browser-host.ps1 -Uninstall`. To restrict cleanup to a particular installation, also pass its absolute native-host `-BinaryPath`. The script removes its registry entries and three native-host/origin manifests, preserving download history. Installer uninstallation attempts matching-host cleanup automatically. Remove extensions through the browser. Registrations require a matching unpacked extension ID; moving the extension may change its ID.

Linux/macOS: use the native host itself for explicit per-user registration, from its permanent installed location:

```sh
./stratafetch-browser-host --register --chrome-id YOUR_CHROME_EXTENSION_ID --firefox
# Optional separate browser IDs: --chromium-id ID --edge-id ID
./stratafetch-browser-host --unregister
```

Select only the browsers you use; `--firefox` registers `stratafetch@local`. On macOS use `StrataFetch.app/Contents/MacOS/stratafetch-browser-host`. Linux honors XDG_CONFIG_HOME for Chromium browsers and uses `~/.mozilla/native-messaging-hosts` for Firefox. macOS uses the official per-user browser folders under `~/Library/Application Support`. Registration refuses another installation's manifests or symbolic-link files; unregister validates ownership and preserves history and unrelated origins. Unregister before moving binaries, then register their new permanent path. Start the matching desktop or service afterward. Linux CLI registration and the actual native-host/IPC flow were exercised; installed-browser, sandboxed-browser and macOS runtime acceptance remain pending. See [platform evidence](PLATFORMS.md).

The native host accepts only `ping` and `add` messages. It requires a running manager; it does not launch it automatically. Downloads go to the OS Downloads folder. No automatic interception, cookies, history collection, per-site rules, protected-service extraction, or stream detection is implemented. A current page URL downloads the page response, not embedded resources. Popup failures are shown; context-menu success/failure appears as a `+`/`!` extension badge and tooltip. Firefox and Chromium scripts are locally tested with mocked APIs; this does not substitute for installed-browser tests.

Phase 2 verified the actual Windows registry/manifests and cleanup, and exercised the real native executable with official framing against the actual IPC service. Ping, add, download identity/status, origin rejection, service-unavailable errors and downloaded bytes passed. This is a native-host test, not a real installed-browser end-to-end test. Browser installations and signing remain experimental. `STRATAFETCH_DOWNLOAD_DIR` can override the download directory for isolated testing or explicit headless configuration.
