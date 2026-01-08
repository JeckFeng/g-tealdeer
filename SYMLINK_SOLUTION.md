# 符号链接方案（推荐）

## 方案说明

不改造代码结构，通过符号链接实现 CLI 和 GUI 共存。

## 实现方式

### 1. 构建

```bash
# 构建 GUI（包含所有功能）
cd frontend/tealdeer-widget
npm run tauri -- build

# 输出：tealdeer_tile (18MB)
```

### 2. 安装

```bash
# 安装 GUI
cp frontend/tealdeer-widget/src-tauri/target/release/tealdeer_tile ~/.local/bin/

# 创建 CLI 符号链接
ln -s ~/.local/bin/tealdeer_tile ~/.local/bin/tldr

# 设置权限
chmod +x ~/.local/bin/tealdeer_tile
```

### 3. 使用

```bash
# GUI
tealdeer_tile

# CLI（通过符号链接）
tldr tar
```

## 优势

- ✅ 无需改造代码
- ✅ 立即可用
- ✅ 用户体验好
- ✅ 维护成本低

## 打包

在 deb/rpm 安装脚本中自动创建符号链接：

```bash
# postinst
ln -sf /usr/bin/tealdeer_tile /usr/bin/tldr
```

## 结论

这是当前最佳方案，Workspace 改造可作为未来 v2.0 计划。
