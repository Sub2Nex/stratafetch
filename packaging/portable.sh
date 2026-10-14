#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Native builds only: the runtime dependencies and macOS icon helper belong to
# the host target. Build separately on Linux x64/arm64 and Intel/Apple Silicon Macs.
case "$(uname -s)" in
  Linux) platform=linux ;;
  Darwin) platform=macos ;;
  *) echo 'Use portable.ps1 on Windows; unsupported Unix host.' >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64) arch=x64 ;;
  arm64|aarch64) arch=arm64 ;;
  *) echo 'Supported architectures: x86_64 and arm64.' >&2; exit 1 ;;
esac
folder="dist/stratafetch-${platform}-${arch}"
# Construct fresh staging, then replace only this script's generated directory.
mkdir -p dist
staging="$(mktemp -d "dist/.stratafetch-${platform}-${arch}.XXXXXX")"
trap 'rm -rf -- "$staging"' EXIT
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml)"
[ -n "$version" ] || { echo 'Cannot determine workspace version.' >&2; exit 1; }
reported="$(target/release/stratafetch --version)"
[ "$reported" = "stratafetch $version" ] || { echo 'CLI version differs from source; rebuild --bins.' >&2; exit 1; }
for binary in stratafetch stratafetch-desktop stratafetch-browser-host; do
  [ -x "target/release/$binary" ] || { echo "Build target/release/$binary first." >&2; exit 1; }
  cp "target/release/$binary" "$staging/"
done
for document in README.md LICENSE-MIT LICENSE-APACHE THIRD-PARTY-LICENSES.txt PRIVACY.md SECURITY.md ARCHITECTURE.md CONTRIBUTING.md CODE_OF_CONDUCT.md ROADMAP.md CHANGELOG.md; do
  cp "$document" "$staging/"
done
cp -R docs extensions packaging "$staging/"
commit="$(git rev-parse HEAD)"
dirty=false
[ -z "$(git status --porcelain --untracked-files=normal)" ] || dirty=true
printf 'StrataFetch %s\nSource commit: %s\nWorking tree modified: %s\nPlatform: %s\nArchitecture: %s\n' "$version" "$commit" "$dirty" "$platform" "$arch" > "$staging/BUILD.txt"
if [ "$platform" = macos ]; then
  bundle="$staging/StrataFetch.app/Contents"
  mkdir -p "$bundle/MacOS" "$bundle/Resources"
  cp "$staging/stratafetch" "$staging/stratafetch-desktop" "$staging/stratafetch-browser-host" "$bundle/MacOS/"
  cargo run --release --locked -p strata-app --example macos_icon -- "$bundle/Resources/stratafetch.icns"
  cat > "$bundle/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>StrataFetch</string>
<key>CFBundleDisplayName</key><string>StrataFetch</string>
<key>CFBundleIdentifier</key><string>org.stratafetch.StrataFetch</string>
<key>CFBundleExecutable</key><string>stratafetch-desktop</string>
<key>CFBundleIconFile</key><string>stratafetch.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>${version%%-*}</string>
<key>CFBundleVersion</key><string>${version%%-*}</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict></plist>
PLIST
  plutil -lint "$bundle/Info.plist"
fi
rm -rf -- "$folder"
mv "$staging" "$folder"
tar -czf "${folder}.tar.gz" -C dist "$(basename "$folder")"
if command -v sha256sum >/dev/null; then
  (cd dist && sha256sum "$(basename "$folder").tar.gz") > "${folder}.SHA256SUMS"
else
  (cd dist && shasum -a 256 "$(basename "$folder").tar.gz") > "${folder}.SHA256SUMS"
fi
printf '%s\n%s\n' "${folder}.tar.gz" "${folder}.SHA256SUMS"
