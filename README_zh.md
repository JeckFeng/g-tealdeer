# Tealdeer-Tile

![teal deer](docs/src/deer.png)

快速、功能丰富的 [tldr](https://github.com/tldr-pages/tldr) 实现，提供 CLI 和桌面 GUI。

[![Crates.io][crates-io-badge]][crates-io] [![GitHub CI][github-actions-badge]][github-actions]

[crates-io]: https://crates.io/crates/tealdeer
[crates-io-badge]: https://img.shields.io/crates/v/tealdeer.svg
[github-actions]: https://github.com/tealdeer-rs/tealdeer/actions?query=workflow%3ACI
[github-actions-badge]: https://github.com/tealdeer-rs/tealdeer/workflows/CI/badge.svg

## 项目简介

Tealdeer-Tile 提供快速访问社区维护的命令行工具帮助页面。它提供：

- **CLI 工具** (`tldr`)：使用 Rust 编写的快速命令行界面
- **桌面应用** (`tealdeer-tile`)：使用 Tauri + Vue 3 构建的现代 GUI
- **自定义页面**：创建和管理你自己的命令文档
- **离线支持**：初始缓存后无需网络连接即可工作

## 特色

### CLI (`tldr`)
- ⚡ **快速**：使用 Rust 编写，性能优异
- 🎨 **彩色输出**：语法高亮，提高可读性
- 📦 **便携**：单个二进制文件，无依赖
- 🔄 **自动更新**：自动更新缓存
- 🌍 **多语言**：支持多种语言
- 🔍 **模糊搜索**：即使输入错误也能找到命令

### 桌面应用 (`tealdeer-tile`)
- 🖥️ **现代 UI**：使用 Vue 3 构建的简洁界面
- 🔍 **搜索与预览**：实时命令搜索和渲染
- ✏️ **自定义页面**：为你的命令创建自定义文档
- 📝 **补丁支持**：使用额外示例扩展现有页面
- 🌓 **深色模式**：支持浅色和深色主题
- 🌐 **国际化**：中英文界面
- ⌨️ **全局热键**：使用键盘快捷键快速访问
- 📌 **系统托盘**：始终可从系统托盘访问

## 从源码编译

### 前置要求

**CLI：**
```bash
# Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**桌面应用（额外）：**
```bash
# Node.js 和 npm
# 从 https://nodejs.org/ 安装

# 系统依赖（Linux）
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator gtk3
```

### 编译 CLI

```bash
# 发布版本（默认）
cargo build --release -p tldr

# 体积优化（Linux/macOS）
strip target/release/tldr

# 二进制文件位置
./target/release/tldr

# 开发者选项（debug）
# cargo build -p tldr
```

### 编译桌面应用

```bash
cd frontend/tealdeer-widget

# 安装依赖
npm install

# 开发模式
npm run tauri -- dev

# 生产构建
npm run tauri -- build

# 输出：src-tauri/target/release/bundle/
```

详细编译说明请参见 [docs/Build_CLI.md](docs/Build_CLI.md)。

## 从 Release 安装

### CLI

**从 crates.io：**
```bash
cargo install tealdeer
```

**从源码安装（自动配置PATH）：**
```bash
# 克隆仓库
git clone https://github.com/tealdeer-rs/tealdeer.git
cd tealdeer

# 运行安装脚本（自动配置 bash/zsh/fish）
./scripts/install_with_path.sh

# 或手动安装
cargo install --path tldr

# 如果 tldr 命令不可用，手动添加到 PATH：
# 对于 bash/zsh：
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # 或 ~/.zshrc
# 对于 fish：
echo 'set -gx PATH $HOME/.cargo/bin $PATH' >> ~/.config/fish/config.fish
```

**从 GitHub Release：**
```bash
# 从 https://github.com/tealdeer-rs/tealdeer/releases 下载二进制文件
# 解压并移动到 PATH
sudo mv tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr
```

### 桌面应用

**从 [Releases](https://github.com/tealdeer-rs/tealdeer/releases) 下载：**

**Linux (deb/rpm)：**
```bash
# 方法 1：直接安装二进制文件（推荐）
cp tealdeer_tile ~/.local/bin/
chmod +x ~/.local/bin/tealdeer_tile

# 方法 2：使用包管理器（Arch Linux 使用 debtap）
yay -S debtap
sudo debtap -u
debtap Tealdeer-Tile_*.deb
sudo pacman -U tealdeer-tile-*.pkg.tar.zst
```

**AppImage：**
```bash
# 解压并运行（无需 FUSE）
./Tealdeer-Tile_*.AppImage --appimage-extract
./squashfs-root/AppRun
```

## 使用方法

### CLI

```bash
# 查看命令
tldr tar

# 更新缓存
tldr --update

# 列出所有命令
tldr --list

# 搜索命令
tldr --search extract

# 查看快捷键页面
tldr --shortcut vim

# 列出所有快捷键页面
tldr --shortcut --list
```

### 桌面应用

```bash
# 启动应用
tealdeer_tile

# 或使用全局热键（默认：Ctrl+Alt+T）
```

**功能：**
- **搜索标签**：搜索和查看命令文档
- **新建页面标签**：创建自定义页面或补丁
- **管理标签**：管理自定义页面和补丁
- **设置标签**：配置应用偏好

## 注意事项

### 自定义页面

自定义页面存储在：
```
~/.local/share/tealdeer-tile/pages/
```

CLI 默认目录：
```
~/.local/share/tealdeer/pages/
```

**文件命名：**
- 自定义页面：`command.page`
- 补丁：`command.patch`
- 禁用：`command.page.disabled`

### 快捷键页面

快捷键页面存储在：
```
~/.local/share/tealdeer-tile/shortcut_pages/
```

CLI 默认目录：
```
~/.local/share/tealdeer/shortcut_pages/
```

**文件命名：**
- 快捷键页面：`vim.page.md`
- 快捷键补丁：`vim.patch.md`
- 禁用：`vim.page.md.disabled`

### 配置

**CLI 配置：** `~/.config/tealdeer/config.toml`

**桌面应用配置：** `~/.local/share/tealdeer-tile/config.toml`

### 已知问题

1. **AppImage FUSE 错误**：手动解压并运行（参见安装部分）
2. **首次启动白屏**：刷新窗口（仅开发模式）
3. **全局热键冲突**：如果默认热键冲突，在设置中更改

### 平台支持

- ✅ Linux（在 Arch Linux 上测试）
- ✅ macOS（仅 CLI，桌面应用未测试）
- ✅ Windows（仅 CLI，桌面应用未测试）

## 文档

- [编译说明](docs/Build_CLI.md)
- [配置指南](docs/src/config.md)
- [自定义页面指南](docs/src/custom_pages.md)

## 贡献

欢迎贡献！请随时提交 issue 或 pull request。

## 许可证

采用以下任一许可证：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

由你选择。

## 致谢

- 基于 dbrgn 的 [tealdeer](https://github.com/tealdeer-rs/tealdeer)
- [tldr-pages](https://github.com/tldr-pages/tldr) 社区
- 使用 [Tauri](https://tauri.app/) 和 [Vue 3](https://vuejs.org/) 构建
