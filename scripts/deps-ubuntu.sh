#!/usr/bin/env bash
# System packages needed to build and run OyNIx with the Qt fetched by
# scripts/fetch-qt.sh (Qt 6.10+ is not packaged by most distributions yet).
set -euo pipefail
sudo apt-get update
sudo apt-get install -y build-essential pkg-config python3-pip \
  libgl1-mesa-dev libegl-dev libvulkan-dev mesa-vulkan-drivers libxkbcommon-dev \
  libxkbcommon-x11-0 libxcb-cursor0 libxcb-icccm4 libxcb-keysyms1 libxcb-image0 \
  libxcb-render-util0 libxcb-xinerama0 libxcb-shape0 libnss3 libxcomposite1 \
  libxdamage1 libxrandr2 libxtst6 libasound2t64 libfontconfig1 libdbus-1-3
command -v cargo >/dev/null || { curl -sSf https://sh.rustup.rs | sh -s -- -y; }
"$(dirname "$0")/fetch-qt.sh"
