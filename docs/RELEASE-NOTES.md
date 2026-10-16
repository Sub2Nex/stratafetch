# StrataFetch v0.1.0-beta.6 — Shared icon and complete Windows updates

Download the full Windows setup for this release. Users upgrading from beta.5 or older should run the setup once to replace older companion files; those older updaters only replace the desktop executable.

- One navy/turquoise download-and-layers icon now identifies native windows, all three Windows executables, the setup/uninstaller and browser extensions. Master PNG, multi-size ICO and GitHub artwork are included in the repository.
- Windows in-app updates now verify and run the full per-user setup package after the desktop shuts down. The desktop, command line, native browser host, bundled extensions and package metadata are updated together. History/settings remain in their existing profile.
- Settings > Browser integration > Repair browser components can download the current release's matching setup or a newer one, without downgrading. Windows setup errors no longer tell Windows users to install Linux packages.
- Separate Chrome, Edge and Firefox submission kits contain ready-to-paste descriptions, publisher/support metadata, permission explanations, privacy answers, reviewer notes and branded media. Store IDs and Firefox signatures are still assigned by the stores. Public store distribution requires configuring the assigned origins and testing the matching native companion.

Browser capture still asks before handoff and falls back to the browser on rejection/failure. Cookies and credentials are not copied. Private downloads, generated URLs, extension packages and webpage exports remain in the browser. Streaming media/DRM bypass is not offered.

No CI or GitHub Actions was used. Native Windows installers remain unsigned. The engineering roadmap records remaining engine, media and broader reliability milestones; this release does not claim IDM feature parity.
