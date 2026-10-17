# Browser capture repair — beta.7 / extension 0.2.4

The live trace demonstrated a pause followed by an external resume within milliseconds. Malwarebytes Browser Guard is installed and its download inspection code independently pauses and resumes downloads; that matches the trace, but does not prove it caused every failure. The old state guard rejected the resumed snapshot before requesting confirmation. A second problem was serializing the prompt behind pause negotiation and target filename updates. Separate GPU window startup added more work, and earlier source patches did not replace all installed native components.

The repair opens native confirmation independently of pause acquisition, from the first eligible onCreated or onDeterminingFilename event. It claims the browser ID once, serializes confirmation dialogs, and bounds pause reclamation to 12 attempts. Transient paused:false does not close the prompt. Identity, unsupported/private files, browser safety decisions and cancellation remain guarded. The extension checks the browser again after consent and sends a commit over the same native port before native network activity starts. The latest redirect URL/name/size/MIME are refreshed at that commit.

If a file completes while the user is deciding, consent adopts the existing regular file after size/path checks, records a completed native job and removes only the browser history entry after acknowledgment. It does not removeFile, copy, execute or download those bytes again. Declining preserves the browser file and history. This avoids losing a completed file if a new network request would fail.

Windows uses a native TaskDialog, including explicit visibility and foreground/topmost placement. When the local service is already running, it serves confirmation over the existing owner-restricted IPC pipe in the resident process. Otherwise the lightweight confirmation mode is used without initializing a GPU renderer. Closing the desktop keeps it in the system tray by default; General settings can turn that behavior off. The tray has Open/Exit and handles Explorer restarts. --background starts directly in the tray, with a visible fallback if tray creation fails. No Windows autostart was enabled.

## Actual installation

Chrome Default loads ID eakfgnhnogmekemikidnkfajbnfkjgph from:

`C:\Users\under\AppData\Local\Programs\StrataFetch\extensions\chromium`

The native host registry points to `C:\Users\under\AppData\Local\Programs\StrataFetch\stratafetch-browser-host.exe`. The installed package is beta.7 and its matching extension is 0.2.4. One Reload of the existing Chrome extension activates the updated JavaScript. Loading another folder creates unnecessary ambiguity.

## Build and deployment

The canonical Git checkout is D:/stratafetch-main/.publish. Run `python scripts/sync-development.py` there before building; it rejects unknown root workspace edits and synchronizes development unpacked background copies. Archived release ZIPs are not modified. Both canonical and root workspaces were built locally with locked Cargo dependencies for x86_64-pc-windows-gnu, using the existing WSL strata-build toolchain and shared cache. No CI/Actions was used.

The three Windows release executables were staged in .publish/target/release and packaged with packaging/portable.ps1. All PE files are checked for the Common Controls v6 activation manifest. Build resources now also apply to Rust test executables, which otherwise failed to load TaskDialogIndirect. packaging/build-installer.py produces the normal setup from that matching payload. scripts/deploy-local-windows.py validates versions/manifests, requires explicit --install, checks executable locks, backs up installed components, installs the matching host/extensions together and verifies every installed file hash. It never edits download state. The old process was stopped and the installed desktop restarted with --background.

## Evidence and limits

- 167 JavaScript regressions pass, including parallel pause/confirmation, external resume, post-consent safety checks, completed-file accept/decline, rollback, concurrent prompts, exclusions and restart recovery.
- Native Windows: 23 library tests, 18 desktop tests and completed-file adoption regression pass. Four archive/identity tests pass; Firefox lint reports no errors/notices/warnings.
- Installed source hashes match .publish, root and the current unpacked package. Native ping reports bridgeVersion beta.7 and running=true.
- The actual installed window was opened and closed; the same resident process remained available. Its native confirmation was observed and accepted in a controlled no-transfer test. The host returned consent, but closing its pipe without commit created no job. Confirmation was hosted in the existing desktop PID, with no extra desktop process.
- Real Chrome accepted-download transfer after Reload remains unverified. Chrome automation is not connected; do not present fixtures or the native test as a full Chrome handoff. No sub-100ms timing measurement is claimed.

Chrome onCreated fires when a download begins, and onDeterminingFilename runs during filename determination. These APIs cannot guarantee interception before the first network byte. A fast file may complete before a pause takes effect; adoption handles that case without a duplicate file. We do not inject broad page scripts, copy cookies, block browser safety checks, or disable security extensions. Lost final native acknowledgment after a commit remains a protocol limitation requiring request-ID reconciliation; bounded retries do not establish universal at-most-once execution.

Official API reference: https://developer.chrome.com/docs/extensions/reference/api/downloads
