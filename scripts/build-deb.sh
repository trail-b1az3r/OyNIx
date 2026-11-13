#!/usr/bin/env bash
# Build a .deb that depends on the distribution's Qt 6 packages.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
VERSION="${VERSION#v}"
ARCH="$(dpkg --print-architecture)"
ROOT="target/deb/oynix_${VERSION}_${ARCH}"
cargo build --release
rm -rf "$ROOT"
PREFIX="$ROOT/usr"
install -Dm755 target/release/oynix "$PREFIX/bin/oynix"
install -Dm644 packaging/oynix.desktop "$PREFIX/share/applications/oynix.desktop"
install -Dm644 assets/icon.svg "$PREFIX/share/icons/hicolor/scalable/apps/oynix.svg"
for s in 48 64 128 256; do install -Dm644 "assets/icon-$s.png" "$PREFIX/share/icons/hicolor/${s}x${s}/apps/oynix.png"; done
install -Dm644 LICENSE "$PREFIX/share/doc/oynix/copyright"
mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<CTRL
Package: oynix
Version: $VERSION
Section: web
Priority: optional
Architecture: $ARCH
Maintainer: OyNIx contributors
Depends: libqt6webenginequick6, libqt6quick6, libqt6gui6, qml6-module-qtwebengine, qml6-module-qtquick, qml6-module-qtquick-controls, qml6-module-qtquick-layouts, qml6-module-qtquick-window, qml6-module-qtquick-shapes, qml6-module-qtquick-dialogs, qml6-module-qtquick-templates, qml6-module-qtqml-workerscript, qml6-module-qtcore, libvulkan1
Provides: www-browser
Description: Fast, private web browser (Rust + QML)
 OyNIx is a native desktop browser with a built-in tracker and ad blocker,
 vertical or compact tabs, a tab overview, reader view, focus mode and
 profiles. Rendering uses Qt Quick on Vulkan 1.2/1.3 or OpenGL ES.
CTRL
dpkg-deb --build --root-owner-group "$ROOT"
echo "Built $ROOT.deb"
