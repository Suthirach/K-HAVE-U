#!/usr/bin/env bash
# Builds "K9s Studio.app" and a .dmg into ./dist (macOS only).
# UNIVERSAL=1 builds one binary that runs on both Apple Silicon and Intel Macs (used by CI).
# Needs: Rust (rustup) and the ../rune checkout (see README). No Rust is needed on machines
# that only run the finished app.
set -euo pipefail

APP_NAME="K9s Studio"
BIN_NAME="k9s-ui"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:/usr/local/opt/rustup/bin:/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"

cd "$ROOT"
BIN="$ROOT/target/release/$BIN_NAME"
if [ "${UNIVERSAL:-0}" = "1" ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  mkdir -p "$ROOT/target/universal"
  BIN="$ROOT/target/universal/$BIN_NAME"
  lipo -create -output "$BIN" \
    "target/aarch64-apple-darwin/release/$BIN_NAME" \
    "target/x86_64-apple-darwin/release/$BIN_NAME"
  ARCH="universal"
else
  cargo build --release
  ARCH="$(uname -m)"
fi

VERSION="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"
DMG="$DIST/K9s-Studio-Mac.dmg"   # fixed name: the in-app updater downloads this exact file

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>com.k9sstudio.app</string>
  <key>CFBundleExecutable</key><string>$BIN_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc signature: lets the app launch on Apple Silicon (not notarized).
codesign --force --deep --sign - "$APP"

# Disk image: one-click installer, the app (drag also works), an Applications shortcut and an uninstaller.
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
# One-click installer: copies the app to Applications and opens it (no dragging needed)
osacompile -o "$STAGE/Install K9s Studio.app" "$ROOT/packaging/macos/Install-K9s-Studio.applescript"
codesign --force --deep --sign - "$STAGE/Install K9s Studio.app" || true
cp "$ROOT/packaging/macos/อ่านก่อนติดตั้ง.txt" "$STAGE/"
# Uninstaller as a real double-click app (no Terminal window)
osacompile -o "$STAGE/Uninstall K9s Studio.app" "$ROOT/packaging/macos/Uninstall-K9s-Studio.applescript"
codesign --force --deep --sign - "$STAGE/Uninstall K9s Studio.app" || true
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

echo "สร้างเสร็จ:"
echo "  $APP"
echo "  $DMG"
