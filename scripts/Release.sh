#!/usr/bin/env bash
# Publish a new version in one command:   ./scripts/Release.sh 0.1.4
#  1. sets the version in Cargo.toml, commits, pushes and tags  -> GitHub builds the Windows installer
#  2. builds the Mac .dmg on this Mac and uploads it to the same GitHub Release
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"

V="${1:-}"
if [ -z "$V" ]; then
  CUR="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
  echo "ใช้: ./scripts/Release.sh <เวอร์ชันใหม่>   (ตอนนี้คือ $CUR)"
  exit 1
fi
V="${V#v}"
command -v gh >/dev/null || { echo "ต้องมี gh ก่อน: brew install gh && gh auth login"; exit 1; }

echo "== 1/3 ตั้งเวอร์ชัน $V แล้ว push =="
sed -i '' "s/^version = \".*\"/version = \"$V\"/" Cargo.toml
cargo update -p klar --offline >/dev/null 2>&1 || true
git add -A
git commit -m "Release v$V" || true
git push
git tag "v$V"
git push origin "v$V"

echo "== 2/3 build ไฟล์ Mac บนเครื่องนี้ (Windows กำลัง build บน GitHub) =="
UNIVERSAL=1 bash packaging/macos/build-app.sh

echo "== 3/3 อัปโหลด Klar-Mac.dmg ขึ้น Release v$V =="
gh release view "v$V" >/dev/null 2>&1 || gh release create "v$V" --title "v$V" --notes "Release v$V"
gh release upload "v$V" dist/Klar-Mac.dmg --clobber

echo
echo "เสร็จฝั่ง Mac แล้ว ไฟล์ Windows จะขึ้นใน Release เองเมื่อ GitHub build เสร็จ (ดูที่แท็บ Actions)"
