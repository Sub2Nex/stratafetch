# Publishing the local repository

The GitHub connector is authenticated, but its available tools do not create repositories. GitHub CLI and the available browser session are unauthenticated. No repository or release was published during this build.

After authenticating GitHub CLI, publish the existing committed source:

```sh
gh auth login
gh repo create stratafetch --public --source=. --remote=origin --push
```

If the name already exists under your account, inspect that repository before choosing a new name or adding a remote. Do not replace an existing project.

Run the Verify workflow on all three platforms. Enable GitHub private vulnerability reporting in repository settings. The Release artifacts workflow is manual and uploads unsigned portable artifacts to Actions; it does not publish a GitHub release automatically.

Only after documented acceptance criteria pass:

```sh
git tag -a v0.1.0 -m 'StrataFetch 0.1.0'
git push origin v0.1.0
gh release create v0.1.0 dist/stratafetch-windows-x64.zip dist/SHA256SUMS --title 'StrataFetch 0.1.0' --notes-file CHANGELOG.md
```

These are future commands, not evidence of a release. Resolve the native launch/application-control blocker, platform validation, browser validation, and security hardening before using them for a stable release.
