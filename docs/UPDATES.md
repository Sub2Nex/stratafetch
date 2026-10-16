# Desktop updates

Use **Tools → Check for updates…** to check manually. Automatic checks are enabled by default, run on startup and every 24 hours while the desktop remains open, and can be disabled in Settings → General or the updates dialog. Checks run off the UI thread. Network errors and GitHub API rate limits leave the installed application unchanged; retry manually later.

The highest newer semantic version from the repository's most recent 100 releases is offered. Stable installations ignore prereleases. Preview installations also receive newer previews and stable releases. Drafts, malformed tags and older versions are ignored.

On Windows x64, choose **Download update**, then **Restart and install**. The desktop downloads the release's `stratafetch-desktop.exe` and `SHA256SUMS`, verifies SHA-256, and stages the executable next to the application. This requires write access to that folder; the updater never requests administrator access. The helper waits for the existing desktop to exit, including normal engine shutdown, retains the previous executable, replaces the desktop and launches it again. Interrupted downloads remain paused on restart. If copying or launching fails, the helper restores the previous executable and attempts to launch it. Helper errors appear in a native dialog.

The `.stratafetch-update-*` directory next to the application retains `previous.exe` for manual recovery. Once the updated application works and the helper has exited, that directory may be removed. If exit is cancelled or takes over two minutes, the helper stops without replacing the desktop. Application-control restrictions can prevent the helper from starting; the running installation remains intact.

Only the desktop executable is replaced. Use the full portable package to update the CLI, browser host, extensions or documentation. Linux, macOS and releases without a compatible executable/checksum offer the Releases page instead of in-app installation. Existing versions without this updater need one manual upgrade to obtain it.

## Publishing compatible releases

1. Bump `[workspace.package].version` in `Cargo.toml`, refresh `Cargo.lock`, and build/test. Tag the release `v<VERSION>`.
2. Run `packaging/portable.ps1` after a native Windows x64 release build. It produces the portable ZIP, `stratafetch-desktop.exe` and `SHA256SUMS`.
3. Upload those exact assets to the same GitHub release in `Sub2Nex/stratafetch`. Prepare a draft so partial assets are not offered. Mark previews as prereleases. Preserve the `--apply-update` helper protocol in future versions.
4. Publish after verifying the package and exercising an upgrade from the previous updater-enabled version on Windows, including active transfers and a failed replacement.

Checksums detect corruption; they come from the same GitHub release and are not an independent publisher signature. Releases remain unsigned. GitHub repository/release access and HTTPS are the trust boundary. No GitHub Actions workflow or recurring billing resource is added.
