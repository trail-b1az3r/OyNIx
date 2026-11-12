#!/usr/bin/env bash
# Build/runtime dependencies on Ubuntu 24.04+ / Debian 13+ / Linux Mint 22+.
set -euo pipefail
sudo apt-get update
sudo apt-get install -y build-essential pkg-config \
  qt6-base-dev qt6-base-dev-tools qt6-declarative-dev qt6-webengine-dev qt6-webengine-dev-tools \
  qml6-module-qtquick qml6-module-qtquick-controls qml6-module-qtquick-layouts \
  qml6-module-qtquick-window qml6-module-qtquick-shapes qml6-module-qtquick-dialogs \
  qml6-module-qtquick-templates qml6-module-qtqml-workerscript qml6-module-qtwebengine \
  qml6-module-qtcore libvulkan-dev mesa-vulkan-drivers
command -v cargo >/dev/null || { curl -sSf https://sh.rustup.rs | sh -s -- -y; }
