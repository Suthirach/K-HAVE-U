#!/usr/bin/env bash
# For developers: build and run the app from source on macOS.
# It runs from a small Klar.app wrapper so the Dock shows the Klar icon instead of "exec".
set -euo pipefail
export PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."

cargo build "$@"

PROFILE="debug"
for a in "$@"; do [ "$a" = "--release" ] && PROFILE="release"; done
APP="target/$PROFILE/Klar.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# Remove first, never overwrite in place: macOS caches the old binary's signature and kills
# ("zsh: killed") a process whose file changed underneath it.
rm -f "$APP/Contents/MacOS/klar"
cp "target/$PROFILE/klar" "$APP/Contents/MacOS/klar"
cp assets/icon/Klar.icns "$APP/Contents/Resources/Klar.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Klar</string>
  <key>CFBundleDisplayName</key><string>Klar</string>
  <key>CFBundleIdentifier</key><string>com.klar.app.dev</string>
  <key>CFBundleExecutable</key><string>klar</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>Klar</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
# Fresh ad-hoc signature for the whole bundle (required on Apple Silicon).
codesign --force --sign - "$APP" >/dev/null 2>&1 || true
# Tell macOS the bundle changed so the Dock picks up a new icon.
touch "$APP"
# Run in this Terminal so logs and errors stay visible here.
exec "$APP/Contents/MacOS/klar"
