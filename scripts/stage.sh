#!/usr/bin/env bash
# Build OyNIx (release) and lay it out under ROOT as an installed tree:
#   ROOT/usr/lib/oynix/{oynix,qt.conf,qt/…}  ROOT/usr/bin/oynix  desktop file, icons, license
# Shared by build-deb.sh and build-pkg.sh.   usage: scripts/stage.sh <root> <license-path>
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$1"; LICENSE_DEST="${2:-usr/share/licenses/oynix/LICENSE}"
QMAKE="${QMAKE:-$(ls -d .qt/*/gcc_64/bin/qmake 2>/dev/null | sort -V | tail -1)}"
[ -n "$QMAKE" ] && [ -x "$QMAKE" ] || { echo "No Qt found: run scripts/fetch-qt.sh or set QMAKE" >&2; exit 1; }
QMAKE="$(realpath "$QMAKE")"

# The binary finds its bundled Qt relative to itself.
# OYNIX_VERSION (exported by build-deb.sh / build-pkg.sh) is baked into the binary.
# Separate target dir: this binary only runs from inside the bundle, so it
# must not replace the developer build in target/release.
export CARGO_TARGET_DIR="$PWD/target/package"
OYNIX_RPATH='$ORIGIN/qt/lib' QMAKE="$QMAKE" cargo build --release

# Always start with a clean staging tree. ROOT may be relative (target/deb/…)
# or absolute; match both forms without ever deleting a makepkg $pkgdir.
case "$ROOT" in
  target/*|*/target/*) rm -rf -- "$ROOT" ;;
esac

APP="$ROOT/usr/lib/oynix"
install -Dm755 "$CARGO_TARGET_DIR/release/oynix" "$APP/oynix"
EXTRA_ELF="$APP/oynix" scripts/bundle-qt.sh "$QMAKE" "$APP/qt"
mkdir -p "$ROOT/usr/bin"
ln -sfn ../lib/oynix/oynix "$ROOT/usr/bin/oynix"
install -Dm644 packaging/oynix.desktop "$ROOT/usr/share/applications/oynix.desktop"
install -Dm644 assets/icon.svg "$ROOT/usr/share/icons/hicolor/scalable/apps/oynix.svg"
for s in 48 64 128 256; do install -Dm644 "assets/icon-$s.png" "$ROOT/usr/share/icons/hicolor/${s}x${s}/apps/oynix.png"; done
install -Dm644 LICENSE "$ROOT/$LICENSE_DEST"
