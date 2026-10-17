# Privacy

The desktop checks GitHub Releases on startup and every 24 hours while running. Disable this in Settings → General or the updates dialog. GitHub receives your IP address and the StrataFetch version in the HTTP user agent; local download history is never included. Manual checks work with automatic checks disabled. Update downloads contact GitHub and its asset delivery hosts after you click Download update.

StrataFetch stores download URLs (including their query strings), destination paths, progress, validators, errors, and checksums in a local SQLite database. Theme, language, destination, and window geometry are stored by eframe locally. No telemetry, tracking, mandatory account, or proprietary cloud is included.

Downloading necessarily contacts the selected resource and its redirects. The selected server sees your IP address and HTTP request. With browser capture enabled, eligible new download URLs, suggested filenames, sizes and MIME types are sent to the local native host for confirmation and handoff. The extension temporarily stores IDs, start times and acknowledged native handoff IDs for downloads it pauses, so it can recover an interrupted handoff; this bookkeeping is removed after normal completion or successful recovery. Failed recovery retains the record for another attempt. Capture can be disabled in the extension popup. Cookies, POST bodies, authentication headers and general browsing history are not collected. Clipboard data is read only when you explicitly paste.

Remove jobs to remove their local history and partial data. Completed files remain. SQLite deletion does not securely erase old pages, backups, or filesystem snapshots. Signed URLs can contain secrets, so do not share state files.

Explicit history exports contain URL query strings and download recipes. Keep exported JSON private. Engine limits and organization preferences are persisted locally. Scheduling does not use a cloud scheduler or OS autostart. Uninstallation preserves download data and removes matching optional native-host registration; browser extensions must be removed separately.

Optional domain exclusion rules are stored in extension-local storage and apply to automatic capture, including subdomains. They are not sent to the native app, download servers or the developer.

The extension also retains its latest handoff error and timestamp locally so failures can be explained in the popup. HTTP/HTTPS URLs and Windows file paths are redacted from this diagnostic; a successful handoff clears it. It is not sent to the developer.

Automatic capture observes request URLs and methods in bounded memory (up to 512 entries, 60 seconds) to exclude POST requests. It reads no request headers, cookies, bodies or page contents. Captured browser transfers are cancelled and their history entries erased before native confirmation; the URL, basename and available size/type are sent to the local host. Start download requests native transfer; Cancel or a failed bridge never restarts the browser. Pending browser IDs and start times are saved for interrupted cleanup, without replay URLs. If a newly captured file finishes before cancellation takes effect, its browser-owned copy is removed through downloads.removeFile before native confirmation. Existing unrelated completed files are never removed. Native transfers require explicit consent and anonymous response validation; session-dependent downloads can fail and require returning to the original page.

The latest 20 handoff state transitions are retained locally: event, time, browser ID, available state/pause/safety flags and acknowledged native ID. These details appear in the extension popup for troubleshooting, contain no filenames, paths or raw URLs, and are not transmitted to the developer. Old temporary filename-wait fingerprints from extension 0.2.3 are removed on startup.

## Language and capture preferences

The selected interface language is saved locally and shared with the authenticated browser companion. On Windows it is also saved in the app-owned HKCU Software/StrataFetch Language preference for setup. Capture file-type and minimum-size filters are stored locally in the extension. No translation service or telemetry is used. Request-method observation requires webRequest and HTTP/HTTPS host access; there is no cookies permission or injected page script. File names, paths and URLs are not translated.
