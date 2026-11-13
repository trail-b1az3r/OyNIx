#!/usr/bin/env bash
# Build and install for the current user (default ~/.local), bundling Qt.
set -euo pipefail
PREFIX="${PREFIX:-$HOME/.local}"
cd "$(dirname "$0")/.."
QMAKE="${QMAKE:-$(ls -d .qt/*/gcc_64/bin/qmake 2>/dev/null | sort -V | tail -1)}"
[ -x "$QMAKE" ] || { echo "No Qt found: run scripts/fetch-qt.sh or set QMAKE"; exit 1; }
QMAKE="$(realpath "$QMAKE")"
export CARGO_TARGET_DIR="$PWD/target/package"
OYNIX_RPATH='$ORIGIN/qt/lib' QMAKE="$QMAKE" cargo build --release
APP="$PREFIX/lib/oynix"
rm -rf "$APP"
install -Dm755 "$CARGO_TARGET_DIR/release/oynix" "$APP/oynix"
EXTRA_ELF="$APP/oynix" scripts/bundle-qt.sh "$QMAKE" "$APP/qt"
mkdir -p "$PREFIX/bin"; ln -sfn "$APP/oynix" "$PREFIX/bin/oynix"
install -Dm644 packaging/oynix.desktop "$PREFIX/share/applications/oynix.desktop"
install -Dm644 assets/icon.svg "$PREFIX/share/icons/hicolor/scalable/apps/oynix.svg"
for s in 48 64 128 256; do install -Dm644 "assets/icon-$s.png" "$PREFIX/share/icons/hicolor/${s}x${s}/apps/oynix.png"; done
command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" || true
echo "Installed to $PREFIX/bin/oynix"
