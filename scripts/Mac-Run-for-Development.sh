#!/usr/bin/env bash
# For developers: run the app from source (cargo run) with the right PATH on macOS.
export PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.." && exec cargo run "$@"
