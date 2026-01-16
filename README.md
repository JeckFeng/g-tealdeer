# Tealdeer-Tile

![teal deer](docs/src/deer.png)

A fast, feature-rich implementation of [tldr](https://github.com/tldr-pages/tldr) with both CLI and desktop GUI.

[![Crates.io][crates-io-badge]][crates-io] [![GitHub CI][github-actions-badge]][github-actions]

[crates-io]: https://crates.io/crates/tealdeer
[crates-io-badge]: https://img.shields.io/crates/v/tealdeer.svg
[github-actions]: https://github.com/tealdeer-rs/tealdeer/actions?query=workflow%3ACI
[github-actions-badge]: https://github.com/tealdeer-rs/tealdeer/workflows/CI/badge.svg

[中文文档](README_zh.md)

## Overview

Tealdeer-Tile provides quick access to community-maintained help pages for command-line tools.

- **CLI Tool** (`tldr`): Fast command-line interface written in Rust
- **Desktop App** (`tealdeer-tile`): Modern GUI built with Tauri + Vue 3
- **Custom Pages**: Create and manage your own command documentation
- **Offline Support**: Works without internet connection after initial cache

## Features

### CLI (`tldr`)
- ⚡ **Fast**: Written in Rust for optimal performance
- 🎨 **Colorized Output**: Syntax highlighting for better readability
- 📦 **Portable**: Single binary with no dependencies
- 🔄 **Auto-update**: Automatic cache updates
- 🌍 **Multi-language**: Support for multiple languages
- 🔍 **Fuzzy Search**: Find commands even with typos
- ⌨️ **Shortcut Pages**: Manage keyboard shortcuts documentation

### Desktop App (`tealdeer-tile`)
- 🖥️ **Modern UI**: Clean interface built with Vue 3
- 🔍 **Search & Preview**: Real-time command search and rendering
- ✏️ **Custom Pages**: Create custom documentation for your commands
- ⌨️ **Shortcut Pages**: Manage keyboard shortcuts separately from commands
- 📝 **Patch Support**: Extend existing pages with additional examples
- 🌓 **Dark Mode**: Light and dark theme support
- 🌐 **i18n**: English and Chinese interface
- 🔐 **Global Hotkey**: Quick access with keyboard shortcut (Ctrl+Alt+T)
- 📌 **System Tray**: Always accessible from system tray

## Build from Source

### Prerequisites

**CLI:**
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Desktop App (additional):**
```bash
# Node.js and npm from https://nodejs.org/

# System dependencies (Linux)
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator gtk3
```

### Build CLI

```bash
# Release build (default)
cargo build --release -p tldr

# Size optimization (Linux/macOS)
strip target/release/tldr

# Binary: ./target/release/tldr
```

### Build Desktop App

```bash
cd frontend/tealdeer-widget
npm install
npm run tauri -- build

# Output: src-tauri/target/release/bundle/
```

For detailed instructions, see [docs/Build_CLI.md](docs/Build_CLI.md).

## Install from Release

### CLI

**From crates.io:**
```bash
cargo install tealdeer
```

**From GitHub Release:**
```bash
# Download from https://github.com/tealdeer-rs/tealdeer/releases
sudo mv tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr
```

### Desktop App

**Download from [Releases](https://github.com/tealdeer-rs/tealdeer/releases)**

**Linux (recommended):**
```bash
# Direct binary installation
cp tealdeer_tile ~/.local/bin/
chmod +x ~/.local/bin/tealdeer_tile
```

**Linux (package):**
```bash
# Arch Linux with debtap
yay -S debtap
sudo debtap -u
debtap Tealdeer-Tile_*.deb
sudo pacman -U tealdeer-tile-*.pkg.tar.zst
```

**AppImage:**
```bash
# Extract and run (no FUSE required)
./Tealdeer-Tile_*.AppImage --appimage-extract
./squashfs-root/AppRun
```

## Usage

### CLI

```bash
tldr tar              # View command
tldr --update         # Update cache
tldr --list           # List all commands
tldr --search extract # Search commands

# Shortcut pages
tldr --shortcut vim   # View keyboard shortcuts
tldr --shortcut --list # List all shortcut pages
```

### Desktop App

```bash
tealdeer_tile         # Launch app
# Or press Ctrl+Alt+T (global hotkey)
```

## Notes

### Custom Pages

Custom pages are stored in:
- Desktop app: `~/.local/share/tealdeer-tile/pages/`
- CLI: `~/.local/share/tealdeer/pages/`

**File naming:**
- Custom page: `command.page`
- Patch: `command.patch`
- Disabled: `command.page.disabled`

### Shortcut Pages

Shortcut pages are stored in:
- Desktop app: `~/.local/share/tealdeer-tile/shortcut_pages/`
- CLI: `~/.local/share/tealdeer/shortcut_pages/`

**File naming:**
- Shortcut page: `vim.page.md`
- Shortcut patch: `vim.patch.md`
- Disabled: `vim.page.md.disabled`

**Example shortcut page:**
```markdown
# vim

> Vim editor keyboard shortcuts

- Save file:

`Ctrl + O`

- Exit editor:

`Ctrl + X`
```

### Configuration

- **CLI config:** `~/.config/tealdeer/config.toml`
- **Desktop app config:** `~/.local/share/tealdeer-tile/config.toml`

### Known Issues

1. **AppImage FUSE Error**: Extract and run manually (see Install section)
2. **First Launch White Screen**: Refresh window (development mode only)
3. **Global Hotkey Conflict**: Change in Settings if default conflicts

### Platform Support

- ✅ Linux (tested on Arch Linux)
- ✅ macOS (CLI only, desktop app untested)
- ✅ Windows (CLI only, desktop app untested)

## Documentation

- [Build Instructions](docs/Build_CLI.md)
- [Configuration Guide](docs/src/config.md)
- [Custom Pages Guide](docs/src/custom_pages.md)
- [Shortcut Pages Guide](SHORTCUT_PAGES_QUICKSTART.md)

## Contributing

Contributions are welcome! Please submit issues or pull requests.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- Based on [tealdeer](https://github.com/tealdeer-rs/tealdeer) by dbrgn
- [tldr-pages](https://github.com/tldr-pages/tldr) community
- Built with [Tauri](https://tauri.app/) and [Vue 3](https://vuejs.org/)
