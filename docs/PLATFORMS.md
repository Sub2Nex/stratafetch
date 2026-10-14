# Native platform support

StrataFetch uses native executables, OS windows and local IPC, with a shared egui interface and download engine. The current development changes add OS integration for Linux and macOS. They do not establish full IDM feature parity or certify every desktop, filesystem, browser or GPU.

## Implementation

- Windows uses per-user named pipes. Linux/macOS use private same-user Unix sockets in `/tmp/stratafetch-UID`, keyed by the canonical state directory. Long application-data paths fit macOS's 104-byte socket limit. Live endpoints are preserved, stale sockets can recover after a crash, and normal shutdown removes the process's socket. Upgrade matching clients and services together; earlier Unix builds used sockets inside the state directory.
- Shortcuts use Ctrl on Windows/Linux and Command on macOS, including select-all and menu/toolbar hints. Selection retains modifier/range behavior.
- Native destination-folder dialogs are available in Add Download and Settings: Windows dialogs, macOS panels, and Linux's desktop file chooser portal. Linux uses a running session D-Bus and a compatible `xdg-desktop-portal` backend, with the library's standard Zenity fallback when the portal is unavailable. Text entry remains available. Cancelled or late dialog results preserve user edits.
- AccessKit platform adapters are enabled. This makes accessibility integration available; screen-reader, keyboard traversal and high-contrast acceptance still require native testing.
- Linux desktop-menu integration installs a per-user launcher and SVG icon with the same app ID as the Wayland window. It preserves other installations and manually edited launchers. No URL interception, autostart or system-wide installation is enabled.
- Unix browser-host registration supports exact Chrome, Chromium, Edge and Firefox extension IDs, official per-user manifest locations, ownership checks and explicit cleanup without deleting download history. Sandboxed browsers (e.g. Snap/Flatpak) may need distribution-specific native-messaging support; these flows are unverified.
- Native macOS packaging creates `StrataFetch.app`, `Info.plist`, executable permissions and original vector icons at 128, 256, 512 and 1024 pixels. Packages are architecture-specific for Intel and Apple Silicon. Signing/notarization is not configured or claimed.

## Linux build and installation

Use stable Rust with rustfmt/clippy. On Debian/Ubuntu, install the standard C/C++ build tools, pkg-config, `libxkbcommon-dev`, `libwayland-dev` and Vulkan runtime/drivers. A GNOME/KDE desktop should provide its own portal backend for folder dialogs.

```sh
cargo build --release --workspace --bins --locked
bash packaging/portable.sh
```

Extract `dist/stratafetch-linux-x64.tar.gz` (or the arm64 package) into a permanent directory, then run its `./stratafetch install-desktop`. The per-user application menu can now launch StrataFetch. Run `./stratafetch install-desktop --uninstall` from the same installation to remove its launcher/icon; binaries, preferences and downloads remain. The desktop requires X11 or Wayland and a compatible Vulkan driver. Moving binaries requires removing the old launcher before installing the new one.

Artifacts built here use Debian 13 x64. The observed desktop ELF references GLIBC_2.39; this artifact is not verified on older distributions. Build on the oldest supported distribution for broader binary compatibility. Linux arm64 is prepared by the scripts but not built/tested here.

## macOS build and installation

Build on each target Mac with Xcode Command Line Tools (`xcode-select --install`) and native stable Rust. The bundle declares macOS 12.0 as its minimum; actual OS/GPU compatibility is pending verification.

```sh
cargo build --release --workspace --bins --locked
bash packaging/portable.sh
```

Extract the architecture-specific macOS archive and place `StrataFetch.app` in a permanent application location, such as `~/Applications`. Launch the app through Finder. CLI and native-host executables are included in `StrataFetch.app/Contents/MacOS` as well as the archive. Register the host only after moving the app to its permanent location. Unsigned local development builds do not establish trusted distribution or notarization; release signing and native acceptance remain outstanding.

## Validation

From the source checkout:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
node --test scripts/browser-tests.mjs
node --test scripts/unix-smoke.mjs
# Validate release CLI/native-host binaries instead of debug binaries:
STRATAFETCH_BIN_DIR="$PWD/target/release" node --test scripts/unix-smoke.mjs
```

The Unix smoke test starts the real service and native host, creates a deliberately long isolated state path, checks exact native frames and rejected origins, downloads from its local range-capable HTTP server, compares every byte/SHA-256, invokes file verification and cleans up its own processes/state. It does not modify installed browsers or normal user history.

On 2026-10-14, all 60 Linux Rust tests, 14 extension tests, strict Clippy, debug/release builds, native X11 rendering with Xvfb, native Wayland rendering with a headless Weston compositor, CLI/IPC/native-host downloads, desktop-file validation and per-user install/uninstall passed locally. Both rendering runs used Mesa's software Vulkan driver. [Linux rendering evidence](screenshot-linux-native.png) is a real release-binary frame with a completed fixture download. Headless rendering does not certify physical GPUs, distribution desktop sessions, DPI transitions, accessibility clients or installed browsers.

The cloud desktop portal denied requests because it could not inspect the calling process root. The standard GTK/Zenity fallback displayed a real native chooser. Its automated expected-directory check failed after the entered path retained extra characters, so complete folder-selection acceptance is still pending; this is not counted as a passing test. State-level tests passed for applying a selected directory and rejecting stale results. Windows/macOS chooser behavior is untested.

A macOS ARM64 `cargo check` was attempted. It stopped at the native C dependency because this Linux environment has neither an Apple-capable compiler nor the macOS SDK. Mac manifest-path tests and ICNS format/image dimensions were checked locally; no macOS binary, Finder launch, native chooser or runtime behavior is claimed. Validate Intel/Apple Silicon Macs, native folder selection, Retina/monitor transitions, sleep/resume, browser registration, accessibility and signing before declaring full macOS support. New Windows features also require a Windows regression run; historical alpha.3 evidence does not validate these changes.
