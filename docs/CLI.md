
**$PATH = "/mnt/data_nvme/code/tealdeer/"**
> [!NOTE]
> - $PATH 这个环境变量只针对该文档


# 开发环境做测试：
```bash
# 1. 进入 GUI 目录
cd $PATH/frontend/tealdeer-widget

# 2. 安装依赖
npm install

# 3. 开发模式(运行下列命令会自动启动GUI)
npm run tauri dev
```

# 在自己电脑上安装GUI(不需要安装CLI也可以正常使用搜索等功能，终端的tldr命令不能使用)

```bash
# 1. 进入 GUI 目录
cd  $PATH/frontend/tealdeer-widget

# 2. 安装依赖
npm install

# 3. 构建 release（下面三种都可以，只是编译的产物不同，推荐第三个命令）
npm run tauri build # 生成所有默认格式（AppImage、deb 等）；
npm run bundle:linux  # 使用':linux '解决`failed to run linuxdeploy`的错误
npm run bundle:linux:deb-rpm # 只生成 deb 和 rpm 包

# 4. 对于编译生成的deb包如何安装？
    - 'cd  $PATH/target/release/bundle/deb/' # 进入deb包目录
    - 'debtap -q Tealdeer-Tile_0.1.0_amd64.deb ' # 转换deb为Arch包
    - 'sudo pacman -U tealdeer-tile-0.1.0-1-x86_64.pkg.tar.zst' # 安装
```

# 在自己电脑上安装tldr的CLI(可以不用装GUI,就能正常使用tldr的cli)
## 使用 cargo install（推荐）

```bash
# 在 Workspace 根目录执行
cd $PATH
cargo install --path tldr
# 验证（可以在任何目录执行）
tldr --version
```