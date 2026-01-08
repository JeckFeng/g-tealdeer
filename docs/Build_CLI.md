# Tealdeer 项目完整 CLI 命令说明文档

## 前后端环境区分

**后端 (Rust CLI)**：
- **开发环境**：使用 `cargo build` 构建调试版本，启用日志和调试符号
- **生产环境**：使用 `cargo build --release` 构建优化版本，禁用调试信息

**前端 (Tauri + Vue)**：
- **开发环境**：使用 `npm run dev` 和 `npm run tauri -- dev` 进行热重载开发
- **生产环境**：使用 `npm run build` 和 `npm run tauri -- build` 构建优化版本

---

# 1. 开发环境

## 1.1 开发环境【前端】

> [!NOTE]
> 前端开发环境下的命令都在 `/mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget` 路径下运行。

### 步骤 1: 安装依赖

```bash
npm install
```

**命令解释**：
- `npm install`：Node.js 包管理器安装命令
- 作用：根据 package.json 安装所有依赖包
- 包括：Vue 3、Tauri API、TypeScript、Vite 等

### 步骤 2: 启动开发服务器（仅前端）

```bash
npm run dev
```

**命令解释**：
- `npm run dev`：启动 Vite 开发服务器
- 实际执行：`vite`
- 提供热重载、快速刷新等开发功能
- 默认端口：http://localhost:5173
- **仅用于前端开发调试，不启动桌面应用**

### 步骤 3: 启动 Tauri 开发模式（前端 + 桌面应用）

```bash
npm run tauri -- dev
```

**命令解释**：
- `npm run tauri -- dev`：启动 Tauri 开发模式
- `--`：参数分隔符
- `dev`：传递给 tauri 命令的参数
- 同时启动前端开发服务器和 Tauri 桌面应用

## 1.2 开发环境【后端】

> [!NOTE]
> 后端开发环境下的命令都在 `/mnt/data_nvme/code/tealdeer` 路径下运行。

### 步骤 1: 调试版本编译

```bash
cargo build
```

**命令解释**：
- `cargo build`：编译 Rust 项目
- 生成调试版本，包含调试符号
- 输出路径：`target/debug/tldr`

### 步骤 2: 启用日志的调试编译

```bash
cargo build --features logging
```

**命令解释**：
- `--features logging`：启用日志功能
- 包含 env_logger 依赖，支持运行时日志输出

### 步骤 3: 设置日志环境变量

```bash
export RUST_LOG=tldr=debug
```

**命令解释**：
- `RUST_LOG=tldr=debug`：设置日志级别
- `tldr`：模块名称
- `debug`：日志级别（trace, debug, info, warn, error）

---

# 2. 生产环境

## 2.1 生产环境【前端】

> [!NOTE]
> 前端生产环境下的命令都在 `/mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget` 路径下运行。

### 步骤 1: 清理前端缓存

```bash
rm -rf node_modules dist src-tauri/target
npm install
```

**命令解释**：
- `rm -rf node_modules dist src-tauri/target`：删除缓存目录
- `npm install`：重新安装依赖，确保版本一致

### 步骤 2: 生产版本构建

```bash
npm run tauri -- build
```

**命令解释**：
- 执行完整的生产构建流程
- 包含前端优化和 Rust 后端编译
- 生成所有支持的安装包格式（deb, rpm, AppImage）

## 2.2 生产环境【后端】

> [!NOTE]
> 后端生产环境下的命令都在 `/mnt/data_nvme/code/tealdeer` 路径下运行。

### 发布版本编译

```bash
cargo build --release
```

**命令解释**：
- `cargo build --release`：编译优化的发布版本
- 启用所有优化选项，禁用调试信息
- 输出路径：`target/release/tldr`

---

# 3. 如何使用生产环境下打包好的产物？

> [!NOTE]
> 使用安装包的命令都在 `/mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget` 路径下运行。

## 3.1 AppImage

AppImage 官方/社区常用的方式是让 AppImage 手动解包后运行。这样做是为了不依赖 FUSE。

```bash
# 解包
./Tealdeer-Tile_0.1.0_amd64.AppImage --appimage-extract

# 运行
./squashfs-root/AppRun
```

## 3.2 deb 安装包

### 安装方法

```bash
# 直接拷贝二进制文件到用户目录
cp src-tauri/target/release/tealdeer_tile ~/.local/bin/

# 赋予执行权限
chmod +x ~/.local/bin/tealdeer_tile
```

### 运行应用

通过上述命令安装后，在终端的任意路径下都可以运行：

```bash
tealdeer_tile
```

### 卸载

```bash
rm ~/.local/bin/tealdeer_tile
```

## 3.3 rpm 安装包

### 在 Arch Linux 上使用 rpm 包

**方法 1：使用 debtap 转换（推荐）**

```bash
# 安装 debtap（从 AUR）
yay -S debtap

# 更新 debtap 数据库（首次使用）
sudo debtap -u

# 转换 rpm 包为 Arch 包
cd src-tauri/target/release/bundle/rpm
debtap Tealdeer-Tile-0.1.0-1.x86_64.rpm

# 安装生成的 pkg.tar.zst 包
sudo pacman -U tealdeer-tile-*.pkg.tar.zst
```

**方法 2：直接使用二进制文件（最简单）**

```bash
# 与 deb 包相同，直接复制二进制文件
cp src-tauri/target/release/tealdeer_tile ~/.local/bin/
chmod +x ~/.local/bin/tealdeer_tile
```

### 卸载

```bash
# 如果使用 debtap 安装
sudo pacman -R tealdeer-tile

# 如果手动安装
rm ~/.local/bin/tealdeer_tile
```

---

# 4. 故障排除

## 4.1 AppImage 构建失败

```bash
# 使用禁用 strip 的构建
npm run bundle:linux
```

## 4.2 权限问题

```bash
# 确保正确的文件权限
sudo chown -R $USER:$USER ~/.config/tealdeer
```

## 4.3 linuxdeploy error 问题

使用 `npm run bundle:linux` 构建，这样会禁用 strip 的构建，应用体积会变大。

或者只构建 deb 和 rpm 包：

```bash
npm run bundle:linux:deb-rpm
```

## 4.4 Cannot mount AppImage, please check your FUSE setup

AppImage 官方/社区常用的方式是让 AppImage 手动解包后运行。这样做是为了不依赖 FUSE。

```bash
# 解包
./Tealdeer-Tile_0.1.0_amd64.AppImage --appimage-extract

# 运行
./squashfs-root/AppRun
```

## 4.5 构建命令区别

### `npm run build`

- **作用**：仅构建前端（Vue 应用）
- **输出**：`dist/` 目录（HTML, CSS, JS）
- **用途**：Web 部署或前端调试
- **不包含**：Rust 后端、桌面应用

### `npm run tauri -- build`

- **作用**：构建完整的 Tauri 桌面应用
- **流程**：
  1. 运行 `npm run build` 构建前端
  2. 编译 Rust 后端 (`cargo build --release`)
  3. 打包成安装包（deb, rpm, AppImage）
- **输出**：
  - 二进制文件：`src-tauri/target/release/tealdeer_tile`
  - 安装包：`src-tauri/target/release/bundle/`

### `npm run build:linux`

- **等同于**：`NO_STRIP=1 tauri build`
- **作用**：与 `npm run tauri -- build` 相同，但禁用二进制文件 strip
- **用途**：解决 linuxdeploy 构建 AppImage 时的兼容性问题
- **缺点**：生成的文件更大（保留调试符号）

### `tauri build`

- **作用**：与 `npm run tauri -- build` 完全相同
- **区别**：直接调用 tauri 命令，不通过 npm scripts

### 使用建议

| 场景 | 推荐命令 |
|------|---------|
| 仅调试前端 | `npm run build` |
| 标准构建桌面应用 | `npm run tauri -- build` |
| AppImage 构建失败 | `npm run build:linux` |
| 只需要 deb/rpm | `npm run bundle:linux:deb-rpm` |

## 4.6 开发命令区别

### `npm run dev`

- **作用**：仅启动前端开发服务器
- **实际执行**：`vite`
- **端口**：http://localhost:5173
- **特点**：
  - 热重载（HMR）
  - 快速刷新
  - 仅用于前端开发
  - **不启动桌面应用**
  - **无法调用 Tauri API**

### `npm run tauri -- dev`

- **作用**：启动完整的 Tauri 开发环境
- **实际执行**：`tauri dev`
- **流程**：
  1. 启动前端开发服务器（Vite）
  2. 编译 Rust 后端（debug 模式）
  3. 启动桌面应用窗口
- **特点**：
  - 前端热重载
  - 后端自动重编译
  - 可调用 Tauri API
  - 完整的桌面应用体验

### 使用建议

| 场景 | 推荐命令 |
|------|---------|
| 仅调试前端 UI | `npm run dev` |
| 调试桌面应用功能 | `npm run tauri -- dev` |
| 测试 Tauri API 调用 | `npm run tauri -- dev` |
| 快速预览前端样式 | `npm run dev` |

---

## 附录：常用命令速查表

### 开发环境

```bash
# 前端开发（仅 UI）
npm run dev

# 桌面应用开发（完整功能）
npm run tauri -- dev

# 后端开发（CLI 工具）
cargo build --features logging
export RUST_LOG=tldr=debug
./target/debug/tldr
```

### 生产环境

```bash
# 构建桌面应用（所有格式）
npm run tauri -- build

# 构建桌面应用（仅 deb/rpm）
npm run bundle:linux:deb-rpm

# 构建后端 CLI
cargo build --release
```

### 安装与运行

```bash
# 安装到用户目录
cp src-tauri/target/release/tealdeer_tile ~/.local/bin/
chmod +x ~/.local/bin/tealdeer_tile

# 运行
tealdeer_tile
```
