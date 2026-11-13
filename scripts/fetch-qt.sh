#!/usr/bin/env bash
# Download Qt 6.11 (with Qt WebEngine) into ./.qt using aqtinstall.
# build.rs picks it up automatically; no system Qt is needed.
set -euo pipefail
cd "$(dirname "$0")/.."
QT_VERSION="${QT_VERSION:-6.11.3}"
command -v aqt >/dev/null 2>&1 || python3 -m aqt --help >/dev/null 2>&1 || python3 -m pip install --user aqtinstall
python3 -m aqt install-qt linux desktop "$QT_VERSION" linux_gcc_64 \
  -m qtwebengine qtwebchannel qtpositioning qtshadertools qtpdf -O .qt
echo "Qt $QT_VERSION installed to .qt/$QT_VERSION/gcc_64"
