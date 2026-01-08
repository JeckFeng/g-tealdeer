# 构建 CLI 工具指南

本文档说明如何从源代码构建 `tldr` CLI 工具。

## 项目架构

本项目采用 Cargo Workspace 架构：

```
tealdeer/
├── Cargo.toml (Workspace)
├── tealdeer-core/ (共享核心库)
├── tldr/ (CLI 工具)
└── frontend/tealdeer-widget/src-tauri/ (GUI 应用)
```

## 前置要求

### 必需
- **Rust 工具链**：1.70 或更高版本
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### 可选（仅用于 GUI）
- **Node.js**：18.x 或更高版本
- **系统依赖**（Linux）：
  ```bash
  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 libayatana-appindicator gtk3
  
  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libgtk-3-dev
  ```

## 构建 CLI 工具

### 方法 1：构建单个包（推荐）

```bash
# 进入项目目录
cd tealdeer

# 构建 CLI（debug 版本）
cargo build -p tldr

# 构建 CLI（release 版本）
cargo build --release -p tldr

# 二进制文件位置
# Debug: ./target/debug/tldr
# Release: ./target/release/tldr
```

### 方法 2：构建整个 Workspace

```bash
# 构建所有包（包括 CLI 和 GUI）
cargo build --workspace

# Release 版本
cargo build --workspace --release
```

### 方法 3：直接安装

```bash
# 从项目目录安装
cargo install --path tldr

# 安装到 ~/.cargo/bin/tldr
```

## 构建选项

### Features

CLI 支持以下 features：

- `default`：默认启用 rustls-with-webpki-roots
- `logging`：启用日志功能
- `native-tls`：使用系统原生 TLS
- `rustls-with-webpki-roots`：使用 rustls + webpki 根证书
- `rustls-with-native-roots`：使用 rustls + 系统根证书

示例：

```bash
# 启用日志功能
cargo build --release -p tldr --features logging

# 使用原生 TLS
cargo build --release -p tldr --no-default-features --features native-tls
```

## 编译时间和大小

### Debug 版本
- **编译时间**：< 1 秒（增量编译）
- **二进制大小**：约 63 MB

### Release 版本
- **编译时间**：约 8-10 秒（增量编译）
- **二进制大小**：约 5.4 MB
- **优化后**：约 4.4 MB（使用 `strip`）

## 优化二进制大小

```bash
# 编译 release 版本
cargo build --release -p tldr

# 使用 strip 移除调试符号
strip target/release/tldr

# 检查大小
ls -lh target/release/tldr
```

## 测试

```bash
# 运行所有测试
cargo test -p tldr

# 运行 workspace 所有测试
cargo test --workspace

# 运行特定测试
cargo test -p tldr test_name
```

## 安装

### 本地安装

```bash
# 复制到用户目录
mkdir -p ~/.local/bin
cp target/release/tldr ~/.local/bin/
chmod +x ~/.local/bin/tldr

# 确保 ~/.local/bin 在 PATH 中
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 系统安装

```bash
# 复制到系统目录（需要 sudo）
sudo cp target/release/tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr
```

## 验证安装

```bash
# 检查版本
tldr --version

# 查看帮助
tldr --help

# 测试功能
tldr tar
```

## 构建 GUI 应用

如果需要构建 GUI 应用：

```bash
# 进入 GUI 目录
cd frontend/tealdeer-widget

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建 release
npm run tauri build

# 输出位置
# Linux: src-tauri/target/release/bundle/deb/
# Linux: src-tauri/target/release/bundle/rpm/
```

## 常见问题

### 1. 编译错误：找不到 tealdeer-core

**原因**：不在 workspace 根目录

**解决**：
```bash
cd /path/to/tealdeer  # 确保在项目根目录
cargo build -p tldr
```

### 2. 链接错误

**原因**：缺少系统依赖

**解决**：
```bash
# 安装必要的开发包
sudo pacman -S base-devel  # Arch Linux
sudo apt install build-essential  # Ubuntu/Debian
```

### 3. 版本不匹配

**原因**：Rust 版本过旧

**解决**：
```bash
rustup update stable
```

## 交叉编译

### Linux → Windows

```bash
# 安装目标
rustup target add x86_64-pc-windows-gnu

# 安装交叉编译工具
sudo pacman -S mingw-w64-gcc  # Arch Linux

# 编译
cargo build --release -p tldr --target x86_64-pc-windows-gnu
```

### Linux → macOS

需要使用 osxcross 工具链（较复杂，不推荐）

## 性能优化

### 编译时优化

在 `Cargo.toml` 中添加：

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### 运行时优化

```bash
# 使用 PGO（Profile-Guided Optimization）
cargo build --release -p tldr
./target/release/tldr --list  # 生成 profile
cargo build --release -p tldr  # 使用 profile 重新编译
```

## 依赖说明

### 核心依赖
- `tealdeer-core`：共享核心库
- `clap`：命令行参数解析
- `env_logger`：日志功能（可选）

### 间接依赖
- `ureq`：HTTP 客户端
- `yansi`：终端颜色
- `serde`：序列化
- `toml`：配置文件解析

## 开发工作流

```bash
# 1. 克隆仓库
git clone https://github.com/your-repo/tealdeer.git
cd tealdeer

# 2. 检查代码
cargo check -p tldr

# 3. 运行测试
cargo test -p tldr

# 4. 开发构建
cargo build -p tldr

# 5. 运行
./target/debug/tldr tar

# 6. Release 构建
cargo build --release -p tldr

# 7. 安装
cargo install --path tldr
```

## 清理

```bash
# 清理构建产物
cargo clean

# 只清理 release 构建
cargo clean --release

# 清理特定包
cargo clean -p tldr
```

## 更多信息

- **项目主页**：https://github.com/tealdeer-rs/tealdeer
- **文档**：https://tealdeer-rs.github.io/tealdeer/
- **问题反馈**：https://github.com/tealdeer-rs/tealdeer/issues

## 版本历史

- **v1.8.1**：Workspace 架构重构
  - 分离核心库（tealdeer-core）
  - 独立 CLI 包（tldr）
  - 共享依赖管理
