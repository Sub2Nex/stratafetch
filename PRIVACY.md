# Privacy

The desktop checks GitHub Releases on startup and every 24 hours while running. Disable this in Settings → General or the updates dialog. GitHub receives your IP address and the StrataFetch version in the HTTP user agent; local download history is never included. Manual checks work with automatic checks disabled. Update downloads contact GitHub and its asset delivery hosts after you click Download update.

StrataFetch stores download URLs (including their query strings), destination paths, progress, validators, errors, and checksums in a local SQLite database. Theme, language, destination, and window geometry are stored by eframe locally. No telemetry, tracking, mandatory account, or proprietary cloud is included.

Downloading necessarily contacts the selected resource and its redirects. The selected server sees your IP address and HTTP request. Browser handoff sends the explicitly selected URL to a local native host; cookies and browsing history are not collected. Clipboard data is read only when you explicitly paste.

Remove jobs to remove their local history and partial data. Completed files remain. SQLite deletion does not securely erase old pages, backups, or filesystem snapshots. Signed URLs can contain secrets, so do not share state files.

Explicit history exports contain URL query strings and download recipes. Keep exported JSON private. Engine limits and organization preferences are persisted locally. Scheduling does not use a cloud scheduler or OS autostart. Uninstallation preserves download data and removes matching optional native-host registration; browser extensions must be removed separately.
