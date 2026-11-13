#!/usr/bin/env bash
# Build a self-contained .deb: OyNIx plus the Qt 6.11 runtime it needs
# (installed under /usr/lib/oynix, so it never conflicts with system Qt).
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
VERSION="${VERSION#v}"
export OYNIX_VERSION="$VERSION"
ARCH="$(dpkg --print-architecture)"
ROOT="target/deb/oynix_${VERSION}_${ARCH}"
scripts/stage.sh "$ROOT" usr/share/doc/oynix/copyright
SIZE=$(du -sk "$ROOT/usr" | cut -f1)
mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<CTRL
Package: oynix
Version: $VERSION
Section: web
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Maintainer: OyNIx contributors
Depends: libc6, libgl1, libegl1, libvulkan1, libxkbcommon0, libxkbcommon-x11-0, libxcb-cursor0, libxcb-icccm4, libxcb-keysyms1, libxcb-image0, libxcb-render-util0, libxcb-shape0, libnss3, libxcomposite1, libxdamage1, libxrandr2, libxtst6, libfontconfig1, libdbus-1-3, libasound2t64 | libasound2
Provides: www-browser
Description: Fast, private web browser (Rust + QML, Qt 6.11)
 OyNIx is a native desktop browser with a built-in tracker and ad blocker,
 Chrome and Firefox extension support, vertical or compact tabs, a tab
 overview, reader view, focus mode and profiles. Rendering uses Qt Quick on
 Vulkan 1.2/1.3 or OpenGL ES. Ships its own Qt 6.11 runtime.
CTRL
dpkg-deb --build --root-owner-group -Zxz "$ROOT"
ls -lh "$ROOT.deb"
