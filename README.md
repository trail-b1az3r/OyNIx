# OyNIx

A fast, private desktop web browser written in **Rust** and **QML**. The interface is drawn by Qt Quick on **Vulkan 1.3 / 1.2**, with **OpenGL ES** as the fallback.

OyNIx's interaction design follows modern macOS-style browsers: quiet chrome, a centred address field showing just the domain, a tab overview, vertical tabs and a focus mode. It ships with tracker and ad blocking switched on and sends no telemetry.

![Vertical tabs, dark theme](docs/screenshot-dark-vertical.png)

| Tab overview | Reader View |
|---|---|
| ![Tab overview](docs/screenshot-overview.png) | ![Reader](docs/screenshot-reader.png) |

## Features

| Area | What you get |
|------|--------------|
| **Tabs** | Three layouts: *Standard* (tab bar under the toolbar), *Compact* (tabs inside the toolbar), *Vertical* (sidebar). Pinned tabs, drag to reorder, mute, duplicate, move to a new window. Reopen closed tabs. |
| **Tab overview** | `Ctrl+Shift+\`: a searchable grid of live tab previews. |
| **Memory** | Inactive tabs are unloaded after a configurable time. Restored sessions load lazily, so only the visible tab loads. Low Power Mode freezes background tabs and turns off animations. |
| **Privacy** | Native content blocker (built-in tracker and ad rules plus your own EasyList, uBlock or hosts lists), per-site report, tracking-parameter stripping, third-party cookie blocking, fingerprinting protection, Global Privacy Control, optional Do Not Track. Privacy Report on the Start Page. |
| **Address field** | Shows the domain when idle. Suggestions from bookmarks, history and your search engine, plus 25 built-in `!bang` shortcuts (`!w`, `!gh`, `!yt`, `!mdn`…). |
| **Reading** | Reader View (`Ctrl+Shift+R`), Reading List, Focus Mode (`Ctrl+Shift+F`, hover the top edge to reveal the toolbar). |
| **Library** | Sidebar with Bookmarks, Reading List and History. Favorites bar. HTML bookmark import and export. |
| **Start Page** | Native and instant: Favorites, Frequently Visited, Privacy Report, Reading List. |
| **Website settings** | Per-site zoom, JavaScript, content blocker, pop-ups and reader default. |
| **Profiles** | Fully isolated profiles (`--profile work`), each running as its own app. Private windows. |
| **Power users** | Developer Tools, user scripts (`*.user.js` with Greasemonkey metadata) and user styles (`*.user.css`), custom user agent, Print to PDF, view source. |
| **Windows** | Frameless window with built-in close/minimise/zoom controls (left or right), or the system title bar. Light, dark or auto theme with eight accent colours. |

## Architecture

```
src/
  main.rs            CLI, GPU selection, Qt/WebEngine start-up
  gpu.rs             Vulkan 1.2/1.3 device probe (ash) → Vulkan | GLES | GL | software
  core/              Pure Rust, no Qt types
    omnibox.rs         input → URL, search engines, !bangs, tracking-param stripping
    blocker.rs         network content blocker + statistics
    library.rs         history, bookmarks, reading list, closed tabs
    settings.rs        persistent preferences + per-site overrides
    scripts.rs         injected JS: reader view, GPC, anti-fingerprinting
    paths.rs           XDG paths per profile
  bridge/
    core.rs            CXX-Qt QML singleton `Core` (properties + invokables)
    native.rs          cxx bridge to cpp/native.cpp
cpp/native.cpp       graphics API, WebEngine init, request interceptor, cookie filter
qml/                 the complete UI (BrowserWindow, AddressField, TabStrip, Sidebar,
                     TabOverview, StartPage, SettingsWindow, popovers, Icon set…)
```

Every network request passes through a C++ `QWebEngineUrlRequestInterceptor`, which calls the Rust blocker synchronously. Matching is a few hash lookups per request.

### Graphics backends

| Flag | Backend |
|------|---------|
| *(default)* | Vulkan if a non-CPU device with Vulkan ≥ 1.2 exists. Otherwise OpenGL ES (Wayland/EGL), otherwise desktop OpenGL. |
| `--gpu vulkan` | Qt Quick RHI on Vulkan (1.3 preferred, 1.2 minimum) |
| `--gpu gles` | OpenGL ES 3 via EGL |
| `--gpu opengl` | Desktop OpenGL |
| `--gpu software` | CPU rasteriser |

You can also set `OYNIX_GPU=vulkan`. `oynix --gpu-info` prints the detected Vulkan device. The active backend is shown under Settings › About.

> **Why no GLFW?** Qt Quick creates and owns its window and graphics context through its RHI layer. GLFW would have to replace that, which breaks QML input, popups and Qt WebEngine's compositor integration. The RHI already gives direct Vulkan and GLES rendering, so GLFW isn't needed.

## Building

Ubuntu 24.04 / Debian 13 / Mint 22 (Qt 6.4 or newer):

```bash
./scripts/deps-ubuntu.sh        # Qt 6 + QML modules + Vulkan loader (+ rustup if needed)
cargo build --release
./target/release/oynix
```

Arch Linux: `pacman -S rust qt6-base qt6-declarative qt6-webengine qt6-shadertools vulkan-icd-loader`.

Install for the current user, or build a Debian package:

```bash
./scripts/install.sh            # → ~/.local/bin/oynix + desktop entry + icons
./scripts/build-deb.sh          # → target/deb/oynix_<version>_<arch>.deb
```

Run the unit tests (omnibox, blocker, library, settings) with `cargo test`.

## Usage

```
oynix [URL]
oynix --private                 private window
oynix --profile work            separate profile
oynix --gpu vulkan|gles|opengl|software
oynix --wayland | --x11
```

### Keyboard shortcuts

| Action | Keys |
|--------|------|
| New tab / window / private window | `Ctrl+T` / `Ctrl+N` / `Ctrl+Shift+N` |
| Close tab / reopen closed tab | `Ctrl+W` / `Ctrl+Shift+T` |
| Address field | `Ctrl+L`, `Ctrl+K`, `F6` |
| Show all tabs | `Ctrl+Shift+\` |
| Sidebar / Bookmarks / History | `Ctrl+Shift+L` / `Ctrl+B` / `Ctrl+H` |
| Reader View / Focus Mode | `Ctrl+Shift+R` / `Ctrl+Shift+F` |
| Find / next / previous | `Ctrl+F` / `Ctrl+G` / `Ctrl+Shift+G` |
| Bookmark / add to Reading List | `Ctrl+D` / `Ctrl+Shift+D` |
| Zoom in / out / reset | `Ctrl+=` / `Ctrl+-` / `Ctrl+0` |
| Switch tab | `Ctrl+Tab`, `Ctrl+Shift+Tab`, `Ctrl+1…9` |
| Downloads / Settings | `Ctrl+J` / `Ctrl+,` |
| Developer Tools | `F12` (enable under Settings › Advanced) |

## Files

| Path | Contents |
|------|----------|
| `~/.config/oynix/<profile>/settings.json` | preferences and per-site settings |
| `~/.config/oynix/<profile>/filters/*.txt` | extra blocklists (Adblock Plus / uBlock / hosts syntax) |
| `~/.config/oynix/<profile>/userscripts/` | `*.user.js`, `*.user.css` |
| `~/.local/share/oynix/<profile>/` | history, bookmarks, reading list, session, cookies |
| `~/.cache/oynix/<profile>/` | HTTP cache |

## License

MIT. See [LICENSE](LICENSE).
