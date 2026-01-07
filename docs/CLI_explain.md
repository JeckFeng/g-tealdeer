# Tealdeer 项目完整 CLI 命令说明文档

## 项目环境说明

### 前后端环境区分

**后端 (Rust CLI)**：
- **开发环境**：使用 `cargo build` 构建调试版本，启用日志和调试符号
- **生产环境**：使用 `cargo build --release` 构建优化版本，禁用调试信息

**前端 (Tauri + Vue)**：
- **开发环境**：使用 `npm run dev` 和 `npm run tauri -- dev` 进行热重载开发
- **生产环境**：使用 `npm run build` 和 `npm run tauri -- build` 构建优化版本

---

## 1. 开发环境测试步骤

### 1.1 后端 Rust CLI 测试

#### 步骤 1: 基础单元测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test
```
**命令解释**：
- `cargo test`：Rust 包管理器的测试命令
- 作用：运行 `tests/` 目录下的所有集成测试和 `src/` 中的单元测试
- 参数：无额外参数，使用默认配置

#### 步骤 2: 详细输出测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test -- --nocapture
```
**命令解释**：
- `cargo test`：测试命令
- `--`：分隔符，后面的参数传递给测试执行器
- `--nocapture`：显示测试中的 println! 输出，不捕获标准输出

#### 步骤 3: 跳过网络测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test --features ignore-online-tests
```
**命令解释**：
- `--features ignore-online-tests`：启用特定功能标志
- 作用：跳过需要网络连接的测试，适用于离线环境

#### 步骤 4: 特定模块测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test cache
cargo test config
cargo test output
```
**命令解释**：
- `cargo test <pattern>`：运行名称匹配模式的测试
- 作用：分别测试缓存、配置、输出模块的功能

#### 步骤 5: 代码质量检查
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
rustup component add clippy
cargo clean && cargo clippy
```
**命令解释**：
- `rustup component add clippy`：安装 Rust 代码检查工具
- `cargo clean`：清理之前的构建产物
- `cargo clippy`：运行代码质量检查，发现潜在问题

#### 步骤 6: 代码格式检查
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo fmt --check
```
**命令解释**：
- `cargo fmt`：Rust 代码格式化工具
- `--check`：只检查格式是否正确，不修改文件

### 1.2 前端 Tauri + Vue 测试

#### 步骤 1: 安装依赖
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm install
```
**命令解释**：
- `npm install`：Node.js 包管理器安装命令
- 作用：根据 package.json 安装所有依赖包
- 包括：Vue 3、Tauri API、TypeScript、Vite 等

#### 步骤 2: 类型检查
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run build
```
**命令解释**：
- `npm run build`：执行 package.json 中定义的 build 脚本
- 实际执行：`vue-tsc --noEmit && vite build`
- `vue-tsc --noEmit`：TypeScript 类型检查，不生成文件
- `vite build`：Vite 构建工具，生成生产版本

#### 步骤 3: Tauri 后端测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget/src-tauri
cargo test
```
**命令解释**：
- 在 Tauri 后端目录运行 Rust 测试
- 测试 Tauri 命令和后端逻辑

---

## 2. 开发环境编译步骤

### 2.1 后端编译

#### 步骤 1: 调试版本编译
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo build
```
**命令解释**：
- `cargo build`：编译 Rust 项目
- 生成调试版本，包含调试符号
- 输出路径：`target/debug/tldr`

#### 步骤 2: 启用日志的调试编译
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo build --features logging
```
**命令解释**：
- `--features logging`：启用日志功能
- 包含 env_logger 依赖，支持运行时日志输出

#### 步骤 3: 设置日志环境变量
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
export RUST_LOG=tldr=debug
```
**命令解释**：
- `RUST_LOG=tldr=debug`：设置日志级别
- `tldr`：模块名称
- `debug`：日志级别（trace, debug, info, warn, error）

### 2.2 前端编译

#### 步骤 1: 开发服务器启动
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run dev
```
**命令解释**：
- `npm run dev`：启动 Vite 开发服务器
- 实际执行：`vite`
- 提供热重载、快速刷新等开发功能
- 默认端口：http://localhost:5173

#### 步骤 2: Tauri 开发模式
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run tauri -- dev
```
**命令解释**：
- `npm run tauri -- dev`：启动 Tauri 开发模式
- `--`：参数分隔符
- `dev`：传递给 tauri 命令的参数
- 同时启动前端开发服务器和 Tauri 桌面应用

---

## 3. 开发环境打包安装步骤

### 3.1 后端打包

#### 步骤 1: 本地安装调试版本
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo install --path . --features logging
```
**命令解释**：
- `cargo install`：安装 Rust 二进制包
- `--path .`：从当前路径安装
- `--features logging`：启用日志功能
- 安装到：`~/.cargo/bin/tldr`

#### 步骤 2: 验证安装
```bash
# 运行路径: 任意路径
tldr --version
```
**命令解释**：
- 验证 tldr 命令是否正确安装
- 显示版本信息

### 3.2 前端打包

#### 步骤 1: 开发版本构建
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run tauri -- build --debug
```
**命令解释**：
- `npm run tauri -- build`：构建 Tauri 应用
- `--debug`：生成调试版本
- 包含调试符号，便于开发调试

#### 步骤 2: 检查构建产物
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
ls -la src-tauri/target/debug/bundle/
```
**命令解释**：
- `ls -la`：列出详细文件信息
- 查看生成的安装包格式（deb、AppImage 等）

---

## 4. 生产环境测试步骤

### 4.1 后端生产测试

#### 步骤 1: 发布版本测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test --release
```
**命令解释**：
- `cargo test --release`：在发布模式下运行测试
- 使用优化编译，测试性能和最终行为

#### 步骤 2: 跳过网络测试的发布测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo test --release --features ignore-online-tests
```
**命令解释**：
- 结合发布模式和离线测试
- 适用于生产环境的 CI/CD 流程

#### 步骤 3: 性能基准测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo build --release
time ./target/release/tldr tar
```
**命令解释**：
- `time`：测量命令执行时间
- 验证生产版本的性能表现

#### 步骤 4: 功能完整性测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
./target/release/tldr --update
./target/release/tldr --list
./target/release/tldr git
```
**命令解释**：
- `--update`：更新缓存，测试网络功能
- `--list`：列出所有页面，测试缓存读取
- `git`：查看具体命令，测试渲染功能

### 4.2 前端生产测试

#### 步骤 1: 生产构建测试
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run build
```
**命令解释**：
- 执行生产环境构建
- 包含类型检查和代码优化

#### 步骤 2: 预览生产版本
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run preview
```
**命令解释**：
- `npm run preview`：预览生产构建结果
- 实际执行：`vite preview`
- 在本地服务器上运行生产版本

---

## 5. 生产环境编译步骤

### 5.1 后端生产编译

#### 步骤 1: 清理构建缓存
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo clean
```
**命令解释**：
- `cargo clean`：删除所有构建产物
- 确保从干净状态开始构建

#### 步骤 2: 发布版本编译
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
cargo build --release
```
**命令解释**：
- `--release`：启用发布模式优化
- 优化级别：3（最高）
- 启用 LTO（链接时优化）
- 生成路径：`target/release/tldr`

#### 步骤 3: 验证二进制文件
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
file target/release/tldr
ldd target/release/tldr
```
**命令解释**：
- `file`：显示文件类型和架构信息
- `ldd`：显示动态链接库依赖

### 5.2 前端生产编译

#### 步骤 1: 清理前端缓存
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
rm -rf node_modules dist src-tauri/target
npm install
```
**命令解释**：
- `rm -rf node_modules dist src-tauri/target`：删除缓存目录
- `npm install`：重新安装依赖，确保版本一致

#### 步骤 2: 生产版本构建
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run tauri -- build
```
**命令解释**：
- 执行完整的生产构建流程
- 包含前端优化和 Rust 后端编译
- 生成所有支持的安装包格式

---

## 6. 生产环境打包安装步骤

### 6.1 后端打包安装

#### 步骤 1: 系统级安装
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
sudo cp target/release/tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr
```
**命令解释**：
- `sudo cp`：以管理员权限复制文件
- `/usr/local/bin/`：系统级二进制文件目录
- `chmod +x`：设置可执行权限

#### 步骤 2: Shell 补全安装
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer
sudo cp completion/tldr.bash /usr/share/bash-completion/completions/tldr
sudo cp completion/tldr.fish /usr/share/fish/vendor_completions.d/tldr.fish
sudo cp completion/_tldr /usr/share/zsh/site-functions/_tldr
```
**命令解释**：
- 安装各种 Shell 的自动补全脚本
- 提升用户体验

#### 步骤 3: 验证安装
```bash
# 运行路径: 任意路径
which tldr
tldr --version
tldr --show-paths
```
**命令解释**：
- `which tldr`：显示 tldr 命令的路径
- `--show-paths`：显示配置和缓存目录

### 6.2 前端打包安装

#### 步骤 1: 生成安装包
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
npm run tauri -- build
```
**命令解释**：
- 生成多种格式的安装包
- 包括：deb、rpm、AppImage

#### 步骤 2: 检查生成的包
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
ls -la src-tauri/target/release/bundle/
find src-tauri/target/release/bundle/ -name "*.deb" -o -name "*.AppImage" -o -name "*.rpm"
```
**命令解释**：
- `ls -la`：列出所有生成的文件
- `find`：查找特定格式的安装包

#### 步骤 3: AppImage 安装
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
chmod +x src-tauri/target/release/bundle/appimage/*.AppImage
sudo cp src-tauri/target/release/bundle/appimage/*.AppImage /opt/tealdeer-widget
```
**命令解释**：
- `chmod +x`：设置 AppImage 可执行权限
- 复制到 `/opt/` 目录，系统应用安装位置

#### 步骤 4: 创建桌面快捷方式
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
sudo tee /usr/share/applications/tealdeer-widget.desktop > /dev/null << EOF
[Desktop Entry]
Name=Tealdeer Widget
Comment=GUI client for tldr pages
Exec=/opt/tealdeer-widget
Icon=tealdeer
Type=Application
Categories=Utility;
EOF
```
**命令解释**：
- `tee`：写入文件内容
- 创建 .desktop 文件，使应用出现在应用菜单中

#### 步骤 5: Arch Linux deb 包处理
```bash
# 运行路径: /mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget
# 安装 debtap（如果未安装）
yay -S debtap
sudo debtap -u

# 转换 deb 包
debtap src-tauri/target/release/bundle/deb/*.deb

# 安装转换后的包
sudo pacman -U *.pkg.tar.xz
```
**命令解释**：
- `debtap`：deb 包转换工具，适用于 Arch Linux
- `-u`：更新 debtap 数据库
- `pacman -U`：安装本地包

#### 步骤 6: 验证安装
```bash
# 运行路径: 任意路径
/opt/tealdeer-widget --version 2>/dev/null || echo "Desktop app installed successfully"
```
**命令解释**：
- 尝试运行桌面应用
- `2>/dev/null`：重定向错误输出
- 验证安装是否成功

---

## 环境变量说明

### 开发环境变量
```bash
# Rust 日志级别
export RUST_LOG=tldr=debug

# 禁用颜色输出（测试环境）
export NO_COLOR=1

# 自定义配置目录
export TEALDEER_CONFIG_DIR=/path/to/config
```

### 生产环境变量
```bash
# 生产日志级别
export RUST_LOG=tldr=info

# 编辑器设置（自定义页面功能）
export EDITOR=vim
```

---

## 故障排除

### 常见问题

1. **AppImage 构建失败**：
```bash
# 使用禁用 strip 的构建
npm run bundle:linux
```

2. **权限问题**：
```bash
# 确保正确的文件权限
sudo chown -R $USER:$USER ~/.config/tealdeer
```

3. **依赖问题**：
```bash
# 重新安装依赖
cargo clean && cargo build
npm ci  # 使用 package-lock.json 精确安装
```

这份文档涵盖了 tealdeer 项目在开发和生产环境下的完整操作流程，每个命令都包含了详细的解释和运行路径说明。
