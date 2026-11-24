#!/usr/bin/env bash
# Double-click: opens the same permission dialog as the "Uninstall Klar" app in the .dmg.
exec osascript "$(cd "$(dirname "$0")/.." && pwd)/packaging/macos/Uninstall-Klar.applescript"
