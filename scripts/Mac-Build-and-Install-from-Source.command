#!/usr/bin/env bash
# For developers: double-click to build Klar from source and install it into /Applications.
# (People who just want to use the app should download Klar-Mac.dmg instead.)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Klar: build จาก source แล้วติดตั้ง =="

for b in /opt/homebrew/bin/brew /usr/local/bin/brew; do
  [ -x "$b" ] && eval "$("$b" shellenv)" && break
done
command -v brew >/dev/null 2>&1 || { echo "ต้องติดตั้ง Homebrew ก่อน: https://brew.sh"; exit 1; }

# Rust toolchain
if ! command -v cargo >/dev/null 2>&1; then
  command -v rustup >/dev/null 2>&1 || brew install rustup
fi
export PATH="/opt/homebrew/opt/rustup/bin:/usr/local/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

# Rune UI framework must sit next to this folder (Cargo.toml uses ../rune)
if [ ! -d "../rune" ]; then
  echo "กำลังโหลด rune ไว้ข้าง ๆ โปรเจกต์..."
  git clone https://github.com/anomalyco/rune.git ../rune
fi

bash packaging/macos/build-app.sh

SRC="dist/Klar.app"
DEST="/Applications"
[ -w "$DEST" ] || { DEST="$HOME/Applications"; mkdir -p "$DEST"; }
rm -rf "$DEST/Klar.app"
cp -R "$SRC" "$DEST/"
xattr -dr com.apple.quarantine "$DEST/Klar.app" 2>/dev/null || true

echo
echo "ติดตั้งเสร็จแล้วที่ $DEST/Klar.app"
open "$DEST/Klar.app"
if [ -t 0 ]; then read -r -n 1 -p "กดปุ่มใดก็ได้เพื่อปิดหน้าต่างนี้..."; echo; fi
