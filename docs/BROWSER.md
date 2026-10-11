# Experimental browser integration

1. Build `stratafetch-browser-host` and keep the executable in a permanent location.
2. In Chrome/Edge enable developer mode on the extensions page, then Load unpacked `extensions/chromium`. Copy its extension ID.
3. Run PowerShell 7: `./packaging/register-browser-host.ps1 -BinaryPath ./target/release/stratafetch-browser-host.exe -ChromiumExtensionId YOUR_EXTENSION_ID`. No administrator access is needed. The script writes per-user Chrome/Edge/Firefox native-host manifests and registry entries. Review it before running.
   For a portable archive, use `-BinaryPath ./stratafetch-browser-host.exe` instead. The registration must point to its permanent installed location.
4. Start StrataFetch desktop or `stratafetch serve`. Open the popup; its status should say Connected. Send a direct link or use the link context menu.
5. Firefox: load `extensions/firefox/manifest.json` as a temporary extension through about:debugging. Permanent distribution requires Mozilla signing; this preview is not signed.

Unregister with the same script and `-Uninstall` (the required parameters are still required). Remove extensions through the browser. Registrations require a matching unpacked extension ID; moving the extension may change its ID.

Linux/macOS: create an official native-host manifest for `org.stratafetch.host` pointing to the compiled executable and place it in your browser's per-user NativeMessagingHosts directory. Populate the state directory's `browser-origins.json` with the exact allowed Chrome origin and/or `stratafetch@local`. These paths and installation flows are not locally verified.

The native host accepts only `ping` and `add` messages. It requires a running manager; it does not launch it automatically. Downloads go to the OS Downloads folder. No automatic interception, cookies, history collection, per-site rules, protected-service extraction, or stream detection is implemented. A current page URL downloads the page response, not embedded resources. Popup failures are shown; context-menu failures are logged in the extension console.
