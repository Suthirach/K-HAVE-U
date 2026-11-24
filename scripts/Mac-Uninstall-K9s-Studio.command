#!/usr/bin/env bash
# Double-click: opens the same permission dialog as the "Uninstall K9s Studio" app in the .dmg.
exec osascript "$(cd "$(dirname "$0")/.." && pwd)/packaging/macos/Uninstall-K9s-Studio.applescript"
