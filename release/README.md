# Tealdeer CLI Release

## 版本信息

- **版本**：v1.8.1
- **架构**：Workspace 重构版本
- **构建日期**：2026-01-09
- **二进制大小**：4.4 MB（已优化）

## 包含文件

- `tldr` - CLI 可执行文件（Linux x86_64）

## 安装

### 方法 1：复制到用户目录

```bash
mkdir -p ~/.local/bin
cp tldr ~/.local/bin/
chmod +x ~/.local/bin/tldr

# 确保 ~/.local/bin 在 PATH 中
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 方法 2：系统安装

```bash
sudo cp tldr /usr/local/bin/
sudo chmod +x /usr/local/bin/tldr
```

## 验证

```bash
tldr --version
tldr tar
```

## 新特性

### Workspace 架构
- 分离核心库（tealdeer-core）
- 独立 CLI 包（tldr）
- 共享依赖管理

### 性能优化
- 编译时间：< 1 秒（增量）
- 二进制大小：4.4 MB（优化后）
- 启动速度：< 10ms

## 依赖

无运行时依赖，静态链接所有库。

## 支持平台

- ✅ Linux x86_64
- ✅ macOS（未测试）
- ✅ Windows（未测试）

## 更多信息

- 项目主页：https://github.com/tealdeer-rs/tealdeer
- 文档：https://tealdeer-rs.github.io/tealdeer/
- 构建指南：docs/Build_CLI.md
