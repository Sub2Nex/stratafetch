# StrataFetch v0.1.0-beta.7 — Browser handoff and Windows tray

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
