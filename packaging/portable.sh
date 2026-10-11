#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
platform="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
folder="dist/stratafetch-${platform}"
mkdir -p "$folder"
cp target/release/stratafetch target/release/stratafetch-desktop target/release/stratafetch-browser-host "$folder/"
cp README.md LICENSE-MIT LICENSE-APACHE THIRD-PARTY-LICENSES.txt PRIVACY.md SECURITY.md ARCHITECTURE.md CONTRIBUTING.md CODE_OF_CONDUCT.md ROADMAP.md CHANGELOG.md "$folder/"
cp -R docs extensions "$folder/"
tar -czf "${folder}.tar.gz" -C dist "$(basename "$folder")"
if command -v sha256sum >/dev/null; then sha256sum "${folder}.tar.gz" > dist/SHA256SUMS; else shasum -a 256 "${folder}.tar.gz" > dist/SHA256SUMS; fi
