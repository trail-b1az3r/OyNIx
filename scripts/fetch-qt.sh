#!/usr/bin/env bash
# Download Qt 6.11 (with Qt WebEngine) into ./.qt using aqtinstall.
# build.rs picks it up automatically; no system Qt is needed.
set -euo pipefail
cd "$(dirname "$0")/.."
QT_VERSION="${QT_VERSION:-6.11.3}"
PY=python3; command -v python >/dev/null && [ -n "${VIRTUAL_ENV:-}" ] && PY=python
$PY -m aqt --help >/dev/null 2>&1 || $PY -m pip install --user aqtinstall
$PY -m aqt install-qt linux desktop "$QT_VERSION" linux_gcc_64 \
  -m qtwebengine qtwebchannel qtpositioning qtshadertools qtpdf -O .qt
echo "Qt $QT_VERSION installed to .qt/$QT_VERSION/gcc_64"
