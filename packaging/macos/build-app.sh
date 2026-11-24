#!/usr/bin/env bash
# Builds "Klar.app" and a .dmg into ./dist (macOS only).
# UNIVERSAL=1 builds one binary that runs on both Apple Silicon and Intel Macs (used by CI).
# Needs: Rust (rustup) and the ../rune checkout (see README). No Rust is needed on machines
# that only run the finished app.
set -euo pipefail

APP_NAME="Klar"
BIN_NAME="klar"
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
DMG="$DIST/Klar-Mac.dmg"   # fixed name: the in-app updater downloads this exact file

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"
cp "$ROOT/assets/icon/Klar.icns" "$APP/Contents/Resources/Klar.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>com.klar.app</string>
  <key>CFBundleExecutable</key><string>$BIN_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>Klar</string>
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
osacompile -o "$STAGE/Install Klar.app" "$ROOT/packaging/macos/Install-Klar.applescript"
codesign --force --deep --sign - "$STAGE/Install Klar.app" || true
cp "$ROOT/packaging/macos/อ่านก่อนติดตั้ง.txt" "$STAGE/"
# Uninstaller as a real double-click app (no Terminal window)
osacompile -o "$STAGE/Uninstall Klar.app" "$ROOT/packaging/macos/Uninstall-Klar.applescript"
codesign --force --deep --sign - "$STAGE/Uninstall Klar.app" || true
rm -f "$DMG"
# hdiutil's own size guess is sometimes too small ("No space left on device"): give it the
# staged size plus headroom, and retry because CI runners occasionally report "Resource busy".
SIZE_MB=$(( $(du -sm "$STAGE" | cut -f1) * 2 + 64 ))
ok=0
for attempt in 1 2 3; do
  if hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -fs HFS+ -size "${SIZE_MB}m" \
       -ov -format UDZO "$DMG" >/dev/null; then
    ok=1
    break
  fi
  echo "hdiutil failed (attempt $attempt), retrying..."
  sleep 5
done
[ "$ok" = 1 ] || { echo "Could not create $DMG"; exit 1; }
rm -rf "$STAGE"

echo "สร้างเสร็จ:"
echo "  $APP"
echo "  $DMG"
