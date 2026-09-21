# gpui-starter

A desktop application boilerplate built on [GPUI](https://github.com/zed-industries/zed), the UI framework from Zed.

## Prerequisites

- Rust nightly (edition 2024)
- macOS is the primary target. Linux on X11/Wayland works but is less tested.

## Quick Start

```sh
cargo build
cargo run
```

On macOS you can also build a signed `.app` bundle for local use:

```sh
bash scripts/macos-dev-app.sh
open "$(bash scripts/macos-dev-app.sh)"
```

### Linux Development

Debian/Ubuntu needs build dependencies installed first. GPUI does not use
WebKit or libzstd (those come up in Tauri instructions); it builds against
Wayland/X11, Vulkan, and FreeType/fontconfig.

```sh
sudo apt update
# Verified on Ubuntu 24.04
sudo apt install -y \
  gcc g++ clang pkg-config \
  libfontconfig-dev libfreetype-dev \
  libwayland-dev wayland-protocols \
  libxkbcommon-x11-dev libx11-xcb-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libssl-dev \
  libvulkan-dev libvulkan1
```

`libvulkan-dev` supplies the build headers. `libvulkan1` is the runtime
loader: a release binary will not start on a machine without it.
`scripts/install-linux-deps.sh` installs both.

#### Nix

`flake.nix` provides reproducible Linux builds. The lockfile is not shipped,
so run `nix flake lock` once to pin nixpkgs.

```sh
nix develop            # dev shell with Vulkan/Wayland/XCB on LD_LIBRARY_PATH
nix build .#default    # release binary in result/bin/gpui-starter
```

## Features

### Core

- Lifecycle state machine covering startup, shutdown, crash handling, and first-run detection
- Single-instance guard over IPC that forwards deep links to the running process
- Custom title bar with drag regions and native traffic-light buttons

### UI

- Collapsible sidebar with page routing
- Status bar
- Cmd+K command palette with fuzzy search over app actions
- Undo/redo stack bound to Cmd+Z / Cmd+Y

### Accessibility

- AccessKit bridge via gpui: screen readers on macOS, Linux, and Windows get labeled landmarks, headings, lists, and controls
- Live regions announce command palette selection and result count, form outcomes, and errors as they change
- Diagnostics reports bridge state and active window counts; the wasm build has no bridge, so accessibility is inactive there

### Data

- SQLite persistence via `rusqlite`
- JSON config with schema migrations
- Secure storage through the OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)

### System

- macOS tray icon and quick-access menu
- Native notifications with an in-app toast fallback and a persistent inbox
- Global hotkey (Alt+Space) plus app-level keybindings
- Deep link handling with single-instance forwarding

### Internationalization

- Fluent translations via `es-fluent`
- English (`en`) and Chinese Simplified (`zh-CN`) locales

### Developer Experience

- File-based logging via `tracing-appender`
- Telemetry with off / local / remote modes behind a consent gate
- Diagnostics page showing live app state and subsystem status
- Integration test harness under `#[cfg(test)]`

## Architecture

See [docs/architecture.md](docs/architecture.md) for the module layout and data flow.

## Marketing Site

The Astro site in [`web/`](web/) runs on Bun and is separate from the desktop app. Boilerplate users who do not want it can delete `web/` and `.github/workflows/deploy-docs.yml` without touching the app.

## Themes

24 built-in themes with hot reloading: drop a JSON file into `themes/` and it is picked up without a restart. The set includes Gruvbox, Tokyonight, Catppuccin, Everforest, Solarized, Ayu, Molokai, Jellybeans, Matrix, Fahrenheit, Hybrid, macOS Classic, and High Contrast (dark and light). Custom themes use the same JSON format; see `themes/` for examples.

## License

[MIT](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, code style, commit format, and the PR process.
