# Tealdeer Widget (Tauri v2)

This project is a Linux-only Tauri v2 desktop UI for tealdeer (tldr client). The UI runs in a Tauri WebView and the backend is a Rust Tauri command layer.

## Requirements
- Linux (primary target: KDE Plasma on Wayland; X11 is best-effort)
- Rust 1.88+ (pinned in `rust-toolchain.toml`)
- Node.js 20+ and npm

## Project Structure
- `src/`: Vue 3 UI
- `src-tauri/`: Rust backend and Tauri config

## Development
```bash
npm install
npm run dev
```

```bash
# Run the desktop app
npm run tauri -- dev
```

```bash
# Build the desktop bundle
npm run tauri -- build
```

## Packaging the Sidecar tealdeer Binary
This app can bundle a tealdeer sidecar for offline use. The config expects base name `bin/tldr`
and Tauri resolves per-target variants automatically.

Place binaries in `src-tauri/bin/` using the target-triple suffix:

- `src-tauri/bin/tldr-x86_64-unknown-linux-gnu`
- `src-tauri/bin/tldr-aarch64-unknown-linux-gnu`

The backend will discover any `tldr*` or `tealdeer*` binary under the resources directory.

## AppImage Bundling Note (linuxdeploy strip error)
On some newer Linux distros, linuxdeploy's bundled `strip` fails on libraries that use `.relr.dyn`.
If AppImage bundling fails with `unknown type [0x13] section .relr.dyn`, disable strip:

```bash
npm run bundle:linux
```

If you only need deb/rpm bundles:

```bash
npm run bundle:linux:deb-rpm
```

## Tests
Rust unit tests live under `src-tauri/src/`.

```bash
cd src-tauri
cargo test
```

Optional integration test (requires a working tealdeer binary with cache):

```bash
TEALDEER_TEST_BIN=/usr/bin/tldr cargo test integration_render_tldr_raw
```

## Usage Notes
- Search uses `tldr --raw` and renders Markdown in the UI.
- Custom pages and patches are written to the configured custom pages directory.
- Sidecar mode uses isolated config/cache/pages under the app data directory.

## Logging
- Backend logs: `rust.log` in the app log directory.
- Frontend logs: `webview.log` in the same directory.
- Log level is `Debug` in dev builds and `Info` in release builds.
- Use the Settings page to open the logs folder or jump directly to each log file.

## FAQ
**Tray icon not visible on Wayland?** Some compositors hide legacy tray icons. Try KDE Plasma
or run under X11.

**Global hotkey not firing?** Some Wayland setups restrict global shortcuts. Use the tray menu
or set the hotkey to empty to disable it.

**Window stacking feels unstable?** Disable “Always on top” in Settings.

## Platform Notes and Known Limitations
- Tray availability depends on the desktop environment; some Wayland compositors may not support it.
- Always-on-top and window decorations vary by compositor; fallback to a normal window is expected.
- Global hotkeys can be blocked by the compositor; provide a tray or menu fallback when needed.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
