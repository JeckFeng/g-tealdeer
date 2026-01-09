# 构建 CLI 工具指南

本文档说明如何从源代码构建 `tldr` CLI 工具。

## 项目架构

本项目采用 Cargo Workspace 架构：

```
tealdeer/                          # ← Workspace 根目录
├── Cargo.toml (Workspace)
├── tealdeer-core/ (共享核心库)
├── tldr/ (CLI 工具)
└── frontend/tealdeer-widget/src-tauri/ (GUI 应用)
```

**重要**：所有 `cargo` 命令都需要在 **Workspace 根目录**（`tealdeer/`）下执行。

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

**在 Workspace 根目录执行**：

```bash
# 1. 进入项目根目录
cd /path/to/tealdeer          # ← 必须在这里

# 2. 构建 CLI（debug 版本）
cargo build -p tldr

# 3. 构建 CLI（release 版本）
cargo build --release -p tldr

# 4. 二进制文件位置
# Debug: ./target/debug/tldr
# Release: ./target/release/tldr
```

**说明**：
- `-p tldr` 指定只构建 `tldr` 包
- 自动编译依赖的 `tealdeer-core`
- 不编译 GUI 包（节省时间）

### 方法 2：通过构建整个 Workspace的方式来构建CLI

**在 Workspace 根目录执行**：

```bash
# 1. 进入项目根目录
cd /path/to/tealdeer          # ← 必须在这里

# 2. 构建所有包（包括 CLI 和 GUI）
cargo build --workspace

# 3. Release 版本
cargo build --workspace --release
```

**说明**：
- 编译所有 workspace 成员
- 包括 `tealdeer-core`、`tldr`、`tealdeer_tile`
- 适合 CI/CD 验证
> [!CAUTION]
> 构建整个Workspace,只是编译构建CLI和GUI使用的后端rust代码，并不会把GUI的前端代码也构 建了


### 方法 3：直接安装CLI

**在 Workspace 根目录执行**：

```bash
# 1. 进入项目根目录
cd /path/to/tealdeer          # ← 必须在这里

# 2. 安装到 ~/.cargo/bin/
cargo install --path tldr

# 3. 验证安装
tldr --version                # ← 可以在任何目录执行
```

**说明**：
- 自动编译 release 版本
- 安装到 `~/.cargo/bin/tldr`
- 可以在任何目录直接使用 `tldr` 命令

> [!CAUTION]
> `cargo install --path tldr`只会安装CLI命令工具，并不会安装GUI界面软件

## 三种方法的区别

| 方法 | 命令 | 编译内容 | 编译时间 | 适用场景 |
|------|------|---------|---------|---------|
| 单个包 | `cargo build -p tldr` | 只编译 CLI | ~1秒 | 开发 CLI |
| Workspace | `cargo build --workspace` | 编译所有包 | ~90秒 | CI/CD 验证 |
| 安装 | `cargo install --path tldr` | 编译并安装 | ~10秒 | 日常使用 |

**执行路径**：所有命令都必须在 **Workspace 根目录**（`tealdeer/`）执行。

> [!NOTE]
>
>
> CLI 支持以下 features：
> ```bash
> # 在 Workspace 根目录执行
> cd /path/to/tealdeer
> 
> # 启用日志功能
> cargo build --release -p tldr --features logging
> 
> # 使用原生 TLS
> cargo build --release -p tldr --no-default-features --features > > > native-tls
> ```
>
> **可用 features**：
>
> - `default`：默认启用 rustls-with-webpki-roots
> - `logging`：启用日志功能
> - `native-tls`：使用系统原生 TLS
> - `rustls-with-webpki-roots`：使用 rustls + webpki 根证书
> - `rustls-with-native-roots`：使用 rustls + 系统根证书
>


## 优化二进制大小

**在 Workspace 根目录执行**：

```bash
# 1. 进入项目根目录
cd /path/to/tealdeer

# 2. 编译 release 版本
cargo build --release -p tldr

# 3. 使用 strip 移除调试符号
strip target/release/tldr

```


## 安装 CLI

### 本地安装

**方法 1：使用 cargo install（推荐）**

```bash
# 在 Workspace 根目录执行
cd /path/to/tealdeer
cargo install --path tldr

# 验证（可以在任何目录执行）
tldr --version
```

**方法 2：手动复制**

```bash
# 在 Workspace 根目录执行
cd /path/to/tealdeer

# 复制到用户目录
mkdir -p ~/.local/bin
cp target/release/tldr ~/.local/bin/
chmod +x ~/.local/bin/tldr

# 确保 ~/.local/bin 在 PATH 中
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# 验证（可以在任何目录执行）
tldr --version
```

### 系统安装

```bash
# 在 Workspace 根目录执行
cd /path/to/tealdeer

# 复制到系统目录（需要 sudo）
sudo cp target/release/tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr

# 验证（可以在任何目录执行）
tldr --version
```

## 验证安装

**可以在任何目录执行**：

```bash
# 检查版本
tldr --version

# 查看帮助
tldr --help

# 测试功能
tldr tar
tldr --list
```

## 构建 GUI 应用

**在 GUI 目录执行**：

```bash
# 1. 进入 GUI 目录
cd /path/to/tealdeer/frontend/tealdeer-widget

# 2. 安装依赖
npm install

# 3. 开发模式
npm run tauri dev

# 4. 构建 release
npm run tauri build # 生成所有默认格式（AppImage、deb 等）；
npm run bundle:linux  # 使用':linux '解决的错误
npm run bundle:linux:deb-rpm # 只生成 deb 和 rpm 包
# 5. 输出位置
# Linux: /path/to/tealdeer/target/release/bundle/deb/
# Linux: /path/to/tealdeer/target/release/bundle/rpm/
```
> [!CAUTION]
> - GUI 构建命令在 GUI 目录执行，不在 Workspace 根目录。
> - 在目录"/path/to/tealdeer/frontend/tealdeer-widget"下执行构建命令
> - 在使用APPImage时，可能会出现FUSE 未安装/内核模块未加载的问题，为解决该问题可以让 AppImage 直接解包后运行：
> ```bash
>  APPIMAGE_EXTRACT_AND_RUN=1 ./Tealdeer-Tile_0.1.0_amd64.AppImage
> ```
> 或者也可以使用手动解包的方式运行：
> ```bash
>  ./Tealdeer-Tile_0.1.0_amd64.AppImage --appimage-extract 
>  ./squashfs-root/AppRun
> ```

## 常见问题

### 1. 编译错误：找不到 tealdeer-core

**错误信息**：
```
error: package `tldr` not found
```

**原因**：不在 workspace 根目录

**解决**：
```bash
# 确保在项目根目录
cd /path/to/tealdeer  # ← 必须在这里
pwd                   # 应该显示 .../tealdeer
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

### 4. 找不到二进制文件

**问题**：编译成功但找不到 `tldr` 命令

**解决**：
```bash
# 方法 1：使用完整路径
cd /path/to/tealdeer
./target/release/tldr --version

# 方法 2：安装到 PATH
cargo install --path tldr
tldr --version  # 可以在任何目录执行
```

## 交叉编译

### Linux → Windows

**在 Workspace 根目录执行**：

```bash
# 1. 进入项目根目录
cd /path/to/tealdeer

# 2. 安装目标
rustup target add x86_64-pc-windows-gnu

# 3. 安装交叉编译工具
sudo pacman -S mingw-w64-gcc  # Arch Linux

# 4. 编译
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

**在 Workspace 根目录执行**：

```bash
cd /path/to/tealdeer

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

**所有命令在 Workspace 根目录执行**：

```bash
# 1. 克隆仓库
git clone https://github.com/your-repo/tealdeer.git
cd tealdeer                    # ← 进入根目录

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

# 8. 使用（可以在任何目录）
tldr tar
```

## 清理

**在 Workspace 根目录执行**：

```bash
cd /path/to/tealdeer

# 清理所有构建产物
cargo clean

# 只清理 release 构建
cargo clean --release

# 清理特定包
cargo clean -p tldr
```

## 路径总结

| 操作 | 执行路径 | 示例 |
|------|---------|------|
| 构建 CLI | Workspace 根目录 | `cd tealdeer && cargo build -p tldr` |
| 构建 Workspace | Workspace 根目录 | `cd tealdeer && cargo build --workspace` |
| 安装 CLI | Workspace 根目录 | `cd tealdeer && cargo install --path tldr` |
| 测试 | Workspace 根目录 | `cd tealdeer && cargo test -p tldr` |
| 清理 | Workspace 根目录 | `cd tealdeer && cargo clean` |
| 构建 GUI | GUI 目录 | `cd frontend/tealdeer-widget && npm run tauri build` |
| 使用 CLI | 任何目录 | `tldr tar` |

**记住**：
- 📁 **构建/测试** → 在 **Workspace 根目录**（`tealdeer/`）
- 🚀 **使用命令** → 在 **任何目录**（安装后）

## 更多信息

- **项目主页**：https://github.com/tealdeer-rs/tealdeer
- **文档**：https://tealdeer-rs.github.io/tealdeer/
- **问题反馈**：https://github.com/tealdeer-rs/tealdeer/issues

## 版本历史

- **v1.8.1**：Workspace 架构重构
  - 分离核心库（tealdeer-core）
  - 独立 CLI 包（tldr）
  - 共享依赖管理
  - **重要**：所有构建命令需在 Workspace 根目录执行
