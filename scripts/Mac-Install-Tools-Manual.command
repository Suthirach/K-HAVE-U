#!/usr/bin/env bash
# Double-click: installs what K9s Studio needs at runtime (kubectl, k9s, gcloud).
# Safe to run more than once; anything already installed is skipped.
set -euo pipefail

echo "== K9s Studio: ติดตั้งเครื่องมือที่ต้องใช้ =="

if ! command -v brew >/dev/null 2>&1; then
  for b in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    [ -x "$b" ] && eval "$("$b" shellenv)" && break
  done
fi
if ! command -v brew >/dev/null 2>&1; then
  echo "ยังไม่มี Homebrew กำลังติดตั้ง (อาจถามรหัสผ่านเครื่อง)..."
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  for b in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    [ -x "$b" ] && eval "$("$b" shellenv)" && break
  done
fi

command -v kubectl >/dev/null 2>&1 || brew install kubectl
command -v k9s >/dev/null 2>&1 || brew install k9s
command -v gcloud >/dev/null 2>&1 || brew install --cask gcloud-cli
command -v gke-gcloud-auth-plugin >/dev/null 2>&1 || gcloud components install gke-gcloud-auth-plugin --quiet

echo
echo "เรียบร้อย: kubectl, k9s, gcloud พร้อมใช้งาน"
echo "เปิด K9s Studio แล้วกด Sign in with Google ได้เลย"
if [ -t 0 ]; then read -r -n 1 -p "กดปุ่มใดก็ได้เพื่อปิดหน้าต่างนี้..."; echo; fi
