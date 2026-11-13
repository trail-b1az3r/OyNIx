#!/usr/bin/env bash
# Copy the parts of the Qt installation OyNIx needs at runtime into DEST
# (DEST/lib, plugins, qml, libexec, resources, translations) and write a
# qt.conf for a binary that lives next to DEST.
#   usage: scripts/bundle-qt.sh <qmake> <dest-dir>
set -euo pipefail
QMAKE="$1"; DEST="$2"
QT="$("$QMAKE" -query QT_INSTALL_PREFIX)"
LIBS="$("$QMAKE" -query QT_INSTALL_LIBS)"
mkdir -p "$DEST"/{lib,plugins,qml,libexec,resources,translations}

PLUGINS=(platforms xcbglintegrations egldeviceintegrations platforminputcontexts platformthemes
         imageformats iconengines tls networkinformation wayland-decoration-client
         wayland-graphics-integration-client wayland-shell-integration generic)
for p in "${PLUGINS[@]}"; do
  [ -d "$QT/plugins/$p" ] && cp -a "$QT/plugins/$p" "$DEST/plugins/"
done
# All QML modules (~18 MB). Hand-picking breaks indirect imports — e.g. the
# QtQuick.Dialogs file/folder dialogs import Qt.labs.folderlistmodel.
cp -a "$QT/qml/." "$DEST/qml/"
rm -rf "$DEST/qml/QtTest" "$DEST/qml/QmlTime"
cp -a "$QT/libexec/QtWebEngineProcess" "$DEST/libexec/"
cp -a "$QT/resources/." "$DEST/resources/"
[ -d "$QT/translations/qtwebengine_locales" ] && cp -a "$QT/translations/qtwebengine_locales" "$DEST/translations/"

# Resolve every Qt (and bundled ICU) library the binaries and plugins need.
mapfile -t ELF < <(find "$DEST/plugins" "$DEST/qml" "$DEST/libexec" -name '*.so*' -o -name QtWebEngineProcess; printf '%s\n' "${EXTRA_ELF:-}")
python3 - "$LIBS" "$DEST/lib" "${ELF[@]}" <<'PY'
import os, shutil, subprocess, sys
libs, out, todo = sys.argv[1], sys.argv[2], [p for p in sys.argv[3:] if p]
seen = set()
while todo:
    f = todo.pop()
    try:
        txt = subprocess.run(["ldd", f], capture_output=True, text=True, env={**os.environ, "LD_LIBRARY_PATH": libs}).stdout
    except Exception:
        continue
    for line in txt.splitlines():
        if "=>" not in line: continue
        path = line.split("=>")[1].split("(")[0].strip()
        if path.startswith(libs) and path not in seen:
            seen.add(path)
            real = os.path.realpath(path)
            name = os.path.basename(path)
            shutil.copy2(real, os.path.join(out, name))
            todo.append(real)
print(f"bundled {len(seen)} Qt libraries")
PY
cat > "$DEST/../qt.conf" <<CONF
[Paths]
Prefix = $(basename "$DEST")
CONF
