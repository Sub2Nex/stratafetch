# StrataFetch v0.1.0-beta.11 — Swedish interface and download controls

Enable Svenska under Settings → Appearance. Desktop menus, advanced settings, transfer/scheduler/update dialogs, tray actions and browser confirmation text use Swedish. The browser popup follows the app language received from its local companion; before connecting it uses the browser UI language. The saved choice is shared with separately launched native hosts and the Windows installer. File names, paths, URLs, protocol identifiers and verbatim third-party diagnostics are preserved.

Identified GET downloads are stopped before choice without waiting for anonymous probing or known size, preventing competing resume handlers from continuing the cancelled stream. Keep in browser restarts through the browser. POST downloads are left untouched. A real Edge extension test verifies a stopped stream during consent, browser restart on decline and POST preservation. Chrome may still briefly show its initial download entry because onCreated fires after the request begins.

Extension 0.2.6 adds capture filters for file extensions and minimum size. A minimum size leaves unknown-size requests in the browser. Filters are checked before pausing and again before handoff; manual links bypass automatic-capture filters. Empty/default filters preserve previous behavior. Request-method observation now requires webRequest and HTTP/HTTPS host access. No cookies or request bodies are read.

Manual and batch additions now reuse an unfinished job for the same URL, file name and destination instead of starting duplicate copies. Completed files remain intact, different explicit names/directories remain separate, and reusing a job preserves its schedule/category. Browser capture keeps separate ownership so failed handoff rollback cannot cancel an older manual job.

Use the full Windows setup to update every component and reload the local extension once. Replace older store upload ZIPs with version 0.2.6. Chrome/Edge store identities still require their assigned IDs; Firefox permanent installation requires Mozilla signing. Native Windows binaries remain unsigned. This release does not establish IDM feature or speed parity, and live Chrome accepted-transfer verification after Reload remains outstanding. No CI or Actions was used.

## Previous beta.10 changes

The Windows setup now installs a dedicated stratafetch.ico, explicitly assigns it to app/uninstall Start menu shortcuts, and writes a quoted icon location with index 0 for Installed apps. Setup refreshes shell icon registrations on upgrade. Installer builds reject payloads without the matching icon. Local development deployment uses the full setup so shell metadata is updated along with the executables.

Browser extension 0.2.5 is unchanged; no new store ZIP upload is needed when that version is already uploaded. Windows executables remain unsigned. This is a Windows packaging fix; download behavior is unchanged from beta.9.

## Previous beta.9 changes

Windows browser confirmations now close when their companion connection disappears or the app exits. This releases blocking dialog threads during shutdown instead of leaving Exit or an update waiting for a click in an abandoned prompt. A connection lost after submission no longer opens a second fallback confirmation. Resident confirmation replies omit download history, so a large history does not delay the consent response.

This release includes the beta.7/beta.8 browser interception and system tray work that was pushed to main but absent from GitHub's published Latest release. Use the Windows setup to update all three executables and bundled extensions. Extension version remains 0.2.5: no extension code changed from beta.8, so an already reloaded 0.2.5 installation needs no new extension upload. Users upgrading from older published releases must update/reload the matching extension.

Windows binaries remain unsigned. Actual Chrome handoff after reloading the extension still requires browser verification; request-level interception before Chrome receives any bytes is not guaranteed by the browser downloads API. Public reproducible requests can be stopped while choosing; private or unverifiable requests retain their browser fallback. No CI or Actions is used.

## Previous beta.8 changes

Extension 0.2.5 adds exclusive ownership for verified public GET downloads. An anonymous bounded range request validates known size, file type and public response characteristics. The original Chrome transfer is then cancelled while confirmation is open. Accepting uses StrataFetch; declining or failure restarts the verified URL through the browser. Unknown-size, query/signed, private and unverifiable requests preserve the original browser transfer. A failed browser cancellation never authorizes a competing native transfer. No browser safety protections are disabled. Completed-file adoption still handles a file that wins the race before cancellation.

## Previous beta.7 changes

Matching extension version: 0.2.4. Native confirmation opens in parallel with browser pause acquisition; bounded pause reclamation handles competing resume events. A file that completes before consent can be adopted without copying, deleting or downloading its bytes again. Native transfer requires a final browser commit after consent and safety checks.

Windows confirmations use a lightweight native dialog. A running tray process serves it directly over local IPC. Closing the main window keeps the app in the tray by default, configurable in General settings; the tray offers Open and Exit. Windows startup is not enabled automatically.

Installed component hashes and resident confirmation/close behavior were verified on Windows. JavaScript, relevant native Rust, adoption, archive identity and Mozilla lint checks pass. Full Chrome accepted-transfer verification after Reload remains outstanding, and sub-100ms timing is not claimed. Chrome download events occur after a request begins, so pre-first-byte interception cannot be guaranteed. No browser security settings were disabled.

Use the full Windows setup to update all components, then Reload the existing extension once. See BROWSER-CAPTURE-REPAIR.md for exact paths, build/deployment details, evidence and remaining protocol limitations. No CI or Actions was used; binaries remain unsigned.

## Previous beta.6 changes

Download the full Windows setup for this release. Users upgrading from beta.5 or older should run the setup once to replace older companion files; those older updaters only replace the desktop executable.

- One navy/turquoise download-and-layers icon now identifies native windows, all three Windows executables, the setup/uninstaller and browser extensions. Master PNG, multi-size ICO and GitHub artwork are included in the repository.
- Windows in-app updates now verify and run the full per-user setup package after the desktop shuts down. The desktop, command line, native browser host, bundled extensions and package metadata are updated together. History/settings remain in their existing profile.
- Settings > Browser integration > Repair browser components can download the current release's matching setup or a newer one, without downgrading. Windows setup errors no longer tell Windows users to install Linux packages.
- Separate Chrome, Edge and Firefox submission kits contain ready-to-paste descriptions, publisher/support metadata, permission explanations, privacy answers, reviewer notes and branded media. Store IDs and Firefox signatures are still assigned by the stores. Public store distribution requires configuring the assigned origins and testing the matching native companion.

Browser capture still asks before handoff and falls back to the browser on rejection/failure. Cookies and credentials are not copied. Private downloads, generated URLs, extension packages and webpage exports remain in the browser. Streaming media/DRM bypass is not offered.

No CI or GitHub Actions was used. Native Windows installers remain unsigned. The engineering roadmap records remaining engine, media and broader reliability milestones; this release does not claim IDM feature parity.
