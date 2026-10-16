#!/usr/bin/env python3
"""Build copy-and-paste submission folders beside the extension archives."""
from pathlib import Path
import argparse
import shutil

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, default=root/'dist/store-submission')
args = parser.parse_args()
summary = 'Send browser downloads to StrataFetch. Choose the desktop app or keep downloading in your browser.'
description = '''Bring browser downloads into StrataFetch, a free, open-source desktop download manager.

When an eligible download starts, StrataFetch asks whether you want to download it in the desktop app or keep it in your browser. The extension pauses the browser transfer during the handoff and cancels it only after the local app acknowledges receiving file data. Declined or failed handoffs resume the browser download.

WHAT YOU CAN DO
• Capture eligible public HTTP and HTTPS file downloads automatically.
• Choose Download with StrataFetch or Keep in browser in the confirmation window.
• Send a link through the right-click menu.
• Paste a direct download URL in the extension popup, or use the current page URL.
• Check the local companion connection from the popup.
• Disable automatic capture or exclude domains at any time.

DESKTOP APP REQUIRED
Install the full StrataFetch Windows setup from https://github.com/Sub2Nex/stratafetch/releases/latest, then open Settings > Browser integration and set up your browser. The extension is the bridge to the installed desktop app; it is not a standalone download engine. Native Linux packages are also available for Linux users.

YOUR CHOICE, WITH BROWSER FALLBACK
Confirmation is required before automatic handoff. Private/incognito downloads, browser extension packages, generated blob/data links and webpage exports stay in the browser. Sites that require browser cookies, credentials or a login session may need to download in the browser. StrataFetch does not copy cookies or bypass access restrictions, DRM or paywalls. Streaming video detection and HLS/DASH capture are not currently included.

PRIVACY
The extension sends a selected download URL and related file metadata to the local StrataFetch app, which connects to that download server. Preferences and temporary handoff recovery identifiers are stored locally. There is no developer telemetry, advertising, sale of data or remote extension code. It does not read your general browsing history, passwords, cookies or page contents.

Source, issues and setup instructions: https://github.com/Sub2Nex/stratafetch
Privacy policy: https://github.com/Sub2Nex/stratafetch/blob/main/PRIVACY.md
'''
permissions = '''SINGLE PURPOSE
Hand off user-selected or confirmed browser file downloads to the installed local StrataFetch download manager, with browser fallback.

downloads: Detect eligible downloads, pause while asking the user, cancel only after the native transfer acknowledges file data, and resume the original download on rejection or failure.
nativeMessaging: Communicate with the registered local StrataFetch native host to submit the download URL/file metadata and obtain connection and handoff status.
contextMenus: Add Download with StrataFetch to the link context menu.
activeTab: Read the current tab URL only when the user invokes Use current page URL in the popup. No page content is extracted.
storage: Retain capture preferences, excluded domains and temporary download ID/start time/native handoff ID records for recovery after a background worker restart.

No host permissions, cookies permission, broad history permission, injected content scripts, remotely hosted JavaScript, eval-based downloads, advertising SDKs or analytics SDKs are requested.
'''
privacy = '''PRIVACY POLICY URL
https://github.com/Sub2Nex/stratafetch/blob/main/PRIVACY.md

DATA HANDLING FOR STORE DISCLOSURES
Download URLs and file metadata (suggested filename, size and content type) pass from the browser to the local desktop companion after the user's action/confirmation. The companion requests that URL from the selected download server. Preferences and recovery identifiers stay on the device. No data is sent to the developer, sold, used for ads or shared with data brokers.

Do not certify that no data is accessed, or that no information leaves the browser. The extension accesses download activity and passes the selected URL/metadata to the native app. The transfer server necessarily receives the download request and its normal connection information. No cookies, credentials, passwords, general browsing history, page contents, health, financial or payment information is intentionally collected.

CHROME / EDGE
Describe the download URL as website/browsing activity when the dashboard's definitions cover URLs. Explain local native messaging in the purpose field. Read the current dashboard definitions before selecting the owner certifications. No remote code is used. Certify sale/advertising/unrelated-purpose restrictions only if you agree to comply with them.

FIREFOX
The manifest declares required data consent categories browsingActivity, websiteActivity and websiteContent for the URL and download metadata handled by the bridge. Explain that these are used locally for the requested download, with no developer analytics. The broad category names do not mean page scraping or general history collection. Keep the manifest consent declaration consistent with Mozilla's current definitions and the published policy.
'''
reviewer = '''No account, payment, license key or credentials are needed to use StrataFetch.

REQUIRED COMPANION
Install the matching full Windows setup from the GitHub release, then use Settings > Browser integration. The public source is in crates/strata-app/src/browser_host.rs, browser_capture.rs and browser_integration.rs. Extension JavaScript is plain, readable source; there is no transpilation, minification, remote code or additional source-build step.

FIRST STORE DRAFT ONLY
The first Chrome/Edge upload assigns a store extension ID. Before requesting review, record that actual ID using packaging/configure-store-id.py, rebuild/install the matching full desktop package and supply the working companion release URL here. The initial ZIP is suitable for creating the draft, but its assigned origin is not automatically trusted by an older desktop installation. Do not submit a bridge that cannot connect to its store-installed origin.
Firefox retains stratafetch@local. Normal persistent installation requires Mozilla signing; the unsigned development XPI is only for temporary testing.

TESTS TO PERFORM IN THE INSTALLED BROWSER
1. Install the extension and companion, register the native host, and open the popup. Check connection status.
2. Enable automatic capture. Download a small public direct file. Choose Keep in browser. Verify the browser download resumes without a native download.
3. Repeat and choose Download with StrataFetch. Verify the desktop opens/focuses and downloads the file; the browser copy cancels after native acknowledgment. Compare the file checksum.
4. Disable capture and start another download; it must stay in the browser.
5. Right-click a public direct-file link and select Download with StrataFetch.
6. Add the fixture domain to exclusions and verify capture is skipped, including subdomains.
7. Temporarily unregister the companion with the supplied unregister command, then try capture. Verify fallback resumes the browser download. Register it again afterwards.
8. Try simultaneous downloads; confirmations should be serialized. Close a confirmation to decline it.
9. Try private mode, a generated blob/data download, an extension package and an authenticated URL returning HTML; these must stay in or fall back to the browser.

Use a controlled public direct-file fixture with known bytes; avoid claiming success based only on popup status. Real installed-browser end-to-end verification is still a release checklist item. Automated browser API fixtures and separate native handoff tests do not substitute for this store-origin test.
'''
stores = {
    'chrome': ('Chrome Web Store', 'stratafetch-chrome-webstore.zip', 'https://chrome.google.com/webstore/devconsole', 'Productivity'),
    'edge': ('Microsoft Edge Add-ons', 'stratafetch-edge-addons.zip', 'https://partner.microsoft.com/dashboard/microsoftedge/overview', 'Productivity'),
    'firefox': ('Firefox Add-ons (AMO)', 'stratafetch-firefox-amo.zip', 'https://addons.mozilla.org/developers/addon/submit/distribution', 'Download Management'),
}
for browser, (name, archive, dashboard, category) in stores.items():
    folder = args.output/browser
    folder.mkdir(parents=True, exist_ok=True)
    fields = {'name.txt': 'StrataFetch', 'short-description.txt': summary, 'full-description.txt': description,
              'permission-justifications.txt': permissions, 'privacy-answers.txt': privacy, 'reviewer-notes.txt': reviewer,
              'release-notes.txt': 'Unified StrataFetch branding and icon sizes. Automatic download handoff with confirmation, browser fallback, domain exclusions and local companion status.',
              'metadata.txt': f'Store: {name}\nUpload ZIP: {archive}\nDashboard: {dashboard}\nPublisher: Sub2Nex\nSupport email: nexoew13.scripter@gmail.com\nHomepage: https://github.com/Sub2Nex/stratafetch\nSupport URL: https://github.com/Sub2Nex/stratafetch/issues\nPrivacy URL: https://github.com/Sub2Nex/stratafetch/blob/main/PRIVACY.md\nCategory: {category} (choose closest available dashboard category)\nLanguage: English (United States)\nPrice: Free\nLicense: MIT OR Apache-2.0\nRequires native desktop companion; no Android/mobile support promised.\n'}
    for filename, content in fields.items():
        (folder/filename).write_text(content.rstrip()+'\n', encoding='utf-8')
    shutil.copyfile(root/'extensions'/('firefox' if browser=='firefox' else 'chromium')/'icons/128.png', folder/'store-icon-128.png')
    shutil.copyfile(root/'assets/branding/icon-512.png', folder/'icon-512.png')
    if browser in ['chrome', 'edge']:
        for asset in ['chrome-small-promo', 'chrome-marquee']:
            shutil.copyfile(root/f'assets/branding/{asset}.png', folder/f'{asset}.png')
    (folder/'START-HERE.md').write_text(f'''# {name} submission

Upload `{archive}` from the parent folder, then paste each adjacent text file into its corresponding dashboard field. `short-description.txt` is {len(summary)} characters. `full-description.txt` explains the companion requirement and limitations. `metadata.txt` supplies contact/support links. Use `store-icon-128.png` for the listing icon.

Review `privacy-answers.txt` against the live form and use `reviewer-notes.txt` for review instructions. Complete identity, legal and account declarations yourself. The store ID, public listing URL and signature are assigned by the store; none has been invented here.

Real screenshots still need capturing from the installed extension: popup with a connected companion; the Download/Keep confirmation; a completed controlled file in the desktop. Do not upload the promotional brand tile as a screenshot or use a fake connection-status mockup. Chrome accepts 1280x800 or 640x400 screenshots; Edge accepts 1280x800 or 640x480. For Firefox, follow the current screenshot field's dimensions. Capture a clean English desktop without private data. Edge reads its short description directly from the uploaded manifest; the ZIP uses the same summary supplied here. Its listing logo can use icon-512.png (square, above the 128px minimum).

Chrome/Edge first draft: obtain the assigned extension ID, record it using the publishing guide, then rebuild the companion and verify the installed store origin before requesting review. Firefox: submit the AMO ZIP and obtain Mozilla signing for persistent installation. See the parent SUBMISSION-GUIDE.md.

Official requirements checked 2026-10-16:
- https://developer.chrome.com/docs/webstore/images
- https://learn.microsoft.com/en-us/microsoft-edge/extensions/publish/publish-extension
- https://extensionworkshop.com/documentation/publish/submitting-an-add-on/
''', encoding='utf-8')
print(f'Created three store-specific listing folders in {args.output}')
