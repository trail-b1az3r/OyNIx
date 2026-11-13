#!/usr/bin/env bash
# Build in release mode and install into a prefix (default: ~/.local).
set -euo pipefail
PREFIX="${PREFIX:-$HOME/.local}"
cd "$(dirname "$0")/.."
cargo build --release
install -Dm755 target/release/oynix            "$PREFIX/bin/oynix"
install -Dm644 packaging/oynix.desktop         "$PREFIX/share/applications/oynix.desktop"
install -Dm644 assets/icon.svg                 "$PREFIX/share/icons/hicolor/scalable/apps/oynix.svg"
for s in 48 64 128 256; do
  install -Dm644 "assets/icon-$s.png" "$PREFIX/share/icons/hicolor/${s}x${s}/apps/oynix.png"
done
command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" || true
echo "Installed to $PREFIX/bin/oynix"
