# 在 Arch Linux 上安装 deb 包

## 方法 1: 使用 debtap（推荐）

```bash
# 1. 安装 debtap（从 AUR）
yay -S debtap
# 或
paru -S debtap

# 2. 更新 debtap 数据库（首次使用）
sudo debtap -u

# 3. 转换 deb 包为 Arch 包
cd src-tauri/target/release/bundle/deb
debtap Tealdeer-Tile_0.1.0_amd64.deb

# 4. 安装生成的 pkg.tar.zst 包
sudo pacman -U tealdeer-tile-*.pkg.tar.zst
```

## 方法 2: 直接使用二进制文件（最简单）

```bash
# 1. 复制二进制文件到系统路径
sudo cp src-tauri/target/release/tealdeer_tile /usr/local/bin/

# 2. 设置执行权限
sudo chmod +x /usr/local/bin/tealdeer_tile

# 3. 运行应用
tealdeer_tile
```

## 方法 3: 手动提取 deb 包

```bash
# 1. 提取 deb 包内容
cd src-tauri/target/release/bundle/deb
ar x Tealdeer-Tile_0.1.0_amd64.deb
tar xf data.tar.gz

# 2. 复制文件到系统
sudo cp -r usr/* /usr/

# 3. 运行应用
tealdeer_tile
```

## 卸载

```bash
# 如果使用 debtap 安装
sudo pacman -R tealdeer-tile

# 如果手动安装
sudo rm /usr/local/bin/tealdeer_tile
```
