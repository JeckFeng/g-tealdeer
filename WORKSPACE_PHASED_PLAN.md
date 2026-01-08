# Workspace 改造分阶段技术方案

## 总体目标

将现有单体项目改造为 Workspace 架构，实现 CLI 和 GUI 分离。

## 改造原则

1. **渐进式**：每个阶段都能独立编译和测试
2. **可回滚**：每个阶段完成后提交 Git，出错可回滚
3. **功能保持**：改造过程中不影响现有功能
4. **最小改动**：每个阶段只改动必要的代码

---

## 阶段 0：准备工作（30 分钟）

### 目标
- 创建备份
- 分析现有代码结构
- 确定依赖关系

### 步骤

#### 0.1 创建 Git 分支
```bash
git checkout -b workspace-refactor
git add -A
git commit -m "Checkpoint: Before workspace refactor"
```

#### 0.2 分析现有代码结构
```bash
# 列出所有源文件
ls -la src/

# 分析依赖关系
cargo tree --depth 1

# 检查 lib.rs 导出
grep "^pub" src/lib.rs
```

#### 0.3 创建备份
```bash
cp -r src src.backup
cp Cargo.toml Cargo.toml.backup
```

### 验收标准
- ✅ Git 分支已创建
- ✅ 代码已备份
- ✅ 依赖关系已记录

---

## 阶段 1：创建 Workspace 结构（1 小时）

### 目标
- 创建 Workspace 根配置
- 保持现有代码可编译

### 步骤

#### 1.1 创建 Workspace Cargo.toml
```bash
# 备份原 Cargo.toml
mv Cargo.toml Cargo.toml.package

# 创建 Workspace Cargo.toml
cat > Cargo.toml << 'EOFTOML'
[workspace]
members = [
    "tealdeer-legacy",
]
resolver = "2"

[workspace.package]
version = "1.8.1"
authors = ["Danilo Bargen <mail@dbrgn.ch>"]
license = "MIT OR Apache-2.0"
edition = "2021"
rust-version = "1.85"

[workspace.dependencies]
anyhow = "1"
app_dirs = { version = "2", package = "app_dirs2" }
clap = { version = "4", features = ["std", "derive", "help", "usage", "cargo", "error-context", "color", "wrap_help"], default-features = false }
env_logger = "0.11"
log = "0.4"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
walkdir = "2"
yansi = "1"
zip = { version = "2", default-features = false, features = ["deflate"] }
EOFTOML
```

#### 1.2 创建 tealdeer-legacy 目录
```bash
mkdir -p tealdeer-legacy
mv src tealdeer-legacy/
mv Cargo.toml.package tealdeer-legacy/Cargo.toml
```

#### 1.3 修改 tealdeer-legacy/Cargo.toml
```bash
# 将依赖改为使用 workspace
sed -i 's/anyhow = "1"/anyhow.workspace = true/' tealdeer-legacy/Cargo.toml
# ... 其他依赖类似
```

#### 1.4 测试编译
```bash
cargo build -p tealdeer-legacy
```

### 验收标准
- ✅ Workspace 结构创建成功
- ✅ `cargo build -p tealdeer-legacy` 编译通过
- ✅ 功能测试通过：`./target/debug/tldr tar`

### 回滚方案
```bash
git reset --hard HEAD
mv src.backup src
mv Cargo.toml.backup Cargo.toml
```

---

## 阶段 2：提取核心库（3-4 小时）

### 目标
- 创建 tealdeer-core 库
- 将核心逻辑移到 tealdeer-core
- tealdeer-legacy 依赖 tealdeer-core

### 步骤

#### 2.1 创建 tealdeer-core 目录结构
```bash
mkdir -p tealdeer-core/src
```

#### 2.2 创建 tealdeer-core/Cargo.toml
```toml
[package]
name = "tealdeer-core"
version.workspace = true
authors.workspace = true
license.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
anyhow.workspace = true
app_dirs.workspace = true
log.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
walkdir.workspace = true
yansi.workspace = true
zip.workspace = true

[features]
default = ["rustls-with-native-roots"]
rustls-with-native-roots = []
```

#### 2.3 识别核心模块（不依赖 CLI）
```bash
# 核心模块列表
# - cache.rs (缓存管理)
# - config.rs (配置)
# - formatter.rs (渲染)
# - types.rs (数据类型，需要移除 clap 依赖)
# - utils.rs (工具函数)
# - api.rs (API 调用)
# - line_iterator.rs (行迭代器)
# - output.rs (输出)
# - extensions.rs (扩展)
```

#### 2.4 复制核心模块到 tealdeer-core
```bash
cp tealdeer-legacy/src/cache.rs tealdeer-core/src/
cp tealdeer-legacy/src/config.rs tealdeer-core/src/
cp tealdeer-legacy/src/formatter.rs tealdeer-core/src/
cp tealdeer-legacy/src/utils.rs tealdeer-core/src/
cp tealdeer-legacy/src/api.rs tealdeer-core/src/
cp tealdeer-legacy/src/line_iterator.rs tealdeer-core/src/
cp tealdeer-legacy/src/output.rs tealdeer-core/src/
cp tealdeer-legacy/src/extensions.rs tealdeer-core/src/
```

#### 2.5 处理 types.rs（移除 clap 依赖）
```bash
# 复制 types.rs
cp tealdeer-legacy/src/types.rs tealdeer-core/src/

# 手动编辑 tealdeer-core/src/types.rs
# 移除 clap::ValueEnum 实现
# 将 PlatformType 改为简单的 enum
```

**修改示例**：
```rust
// 原代码（依赖 clap）
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PlatformType {
    Linux,
    MacOS,
    Windows,
    Common,
}

// 修改后（不依赖 clap）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformType {
    Linux,
    MacOS,
    Windows,
    Common,
}

impl PlatformType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "linux" => Some(Self::Linux),
            "macos" | "osx" => Some(Self::MacOS),
            "windows" => Some(Self::Windows),
            "common" => Some(Self::Common),
            _ => None,
        }
    }
}
```

#### 2.6 创建 tealdeer-core/src/lib.rs
```rust
//! Tealdeer Core Library

pub mod cache;
pub mod config;
pub mod formatter;
pub mod types;
pub mod utils;
pub mod api;
pub mod line_iterator;
pub mod output;
pub mod extensions;

// Re-export commonly used types
pub use cache::Cache;
pub use config::Config;
pub use types::{LineType, PlatformType};
```

#### 2.7 更新 Workspace members
```toml
# Cargo.toml
[workspace]
members = [
    "tealdeer-core",
    "tealdeer-legacy",
]
```

#### 2.8 修改 tealdeer-legacy 依赖 tealdeer-core
```toml
# tealdeer-legacy/Cargo.toml
[dependencies]
tealdeer-core = { path = "../tealdeer-core" }
# ... 其他依赖
```

#### 2.9 修改 tealdeer-legacy 导入
```bash
# 将 use crate:: 改为 use tealdeer_core::
sed -i 's/use crate::cache/use tealdeer_core::cache/g' tealdeer-legacy/src/*.rs
sed -i 's/use crate::config/use tealdeer_core::config/g' tealdeer-legacy/src/*.rs
# ... 其他模块类似
```

#### 2.10 测试编译
```bash
# 编译核心库
cargo build -p tealdeer-core

# 编译 legacy
cargo build -p tealdeer-legacy

# 功能测试
./target/debug/tldr tar
```

### 验收标准
- ✅ `cargo build -p tealdeer-core` 编译通过
- ✅ `cargo build -p tealdeer-legacy` 编译通过
- ✅ 功能测试通过
- ✅ 所有测试通过：`cargo test`

### 回滚方案
```bash
git reset --hard HEAD
```

---

## 阶段 3：创建独立 CLI（2 小时）

### 目标
- 创建新的 tldr CLI 包
- 使用 tealdeer-core
- 保持与原 CLI 功能一致

### 步骤

#### 3.1 创建 tldr 目录
```bash
mkdir -p tldr/src
```

#### 3.2 创建 tldr/Cargo.toml
```toml
[package]
name = "tldr"
version.workspace = true
authors.workspace = true
license.workspace = true
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "tldr"
path = "src/main.rs"

[dependencies]
tealdeer-core = { path = "../tealdeer-core" }
clap.workspace = true
anyhow.workspace = true
env_logger.workspace = true

[features]
default = []
logging = []
```

#### 3.3 创建 tldr/src/main.rs
```bash
# 复制 tealdeer-legacy 的 main.rs 和 cli.rs
cp tealdeer-legacy/src/main.rs tldr/src/
cp tealdeer-legacy/src/cli.rs tldr/src/

# 修改导入
sed -i 's/use crate::/use tealdeer_core::/g' tldr/src/*.rs
```

#### 3.4 处理 CLI 特定代码
```rust
// tldr/src/main.rs
use tealdeer_core::{Cache, Config};
mod cli;

fn main() -> anyhow::Result<()> {
    // CLI 逻辑
    let args = cli::parse_args();
    // ... 使用 tealdeer_core
    Ok(())
}
```

#### 3.5 更新 Workspace members
```toml
[workspace]
members = [
    "tealdeer-core",
    "tealdeer-legacy",
    "tldr",
]
```

#### 3.6 测试编译
```bash
cargo build --release -p tldr

# 测试功能
./target/release/tldr tar
./target/release/tldr --update
./target/release/tldr --list
```

### 验收标准
- ✅ `cargo build -p tldr` 编译通过
- ✅ CLI 所有功能正常
- ✅ 二进制文件大小合理（~6MB）
- ✅ 启动速度快（~10ms）

---

## 阶段 4：修改 GUI 使用核心库（1-2 小时）

### 目标
- GUI 使用 tealdeer-core
- 移除对 tealdeer-legacy 的依赖

### 步骤

#### 4.1 更新 GUI Cargo.toml
```bash
cd frontend/tealdeer-widget/src-tauri

# 修改依赖
sed -i 's|tealdeer = { path = "../../.." }|tealdeer-core = { path = "../../../tealdeer-core" }|' Cargo.toml
```

#### 4.2 更新 GUI 导入
```bash
find src -name "*.rs" -exec sed -i 's/use tealdeer::/use tealdeer_core::/g' {} \;
```

#### 4.3 更新 Workspace members
```toml
[workspace]
members = [
    "tealdeer-core",
    "tldr",
    "frontend/tealdeer-widget/src-tauri",
]
```

#### 4.4 测试编译
```bash
cd frontend/tealdeer-widget
npm run tauri -- build
```

### 验收标准
- ✅ GUI 编译通过
- ✅ GUI 所有功能正常
- ✅ 可以搜索和显示页面

---

## 阶段 5：清理和优化（1 小时）

### 目标
- 删除 tealdeer-legacy
- 更新文档
- 优化构建配置

### 步骤

#### 5.1 删除 tealdeer-legacy
```bash
rm -rf tealdeer-legacy
```

#### 5.2 更新 Workspace members
```toml
[workspace]
members = [
    "tealdeer-core",
    "tldr",
    "frontend/tealdeer-widget/src-tauri",
]
```

#### 5.3 更新 README
```bash
# 更新构建说明
# 更新安装说明
```

#### 5.4 测试完整构建
```bash
# 构建所有包
cargo build --release --workspace

# 构建 GUI
cd frontend/tealdeer-widget
npm run tauri -- build
```

### 验收标准
- ✅ 所有包编译通过
- ✅ 所有测试通过
- ✅ 文档已更新

---

## 阶段 6：打包和发布（1 小时）

### 目标
- 创建安装包
- 测试安装流程

### 步骤

#### 6.1 构建 CLI
```bash
cargo build --release -p tldr
cp target/release/tldr ~/.local/bin/
```

#### 6.2 构建 GUI
```bash
cd frontend/tealdeer-widget
npm run tauri -- build
cp src-tauri/target/release/tealdeer_tile ~/.local/bin/
```

#### 6.3 测试安装
```bash
tldr tar
tealdeer_tile
```

### 验收标准
- ✅ CLI 和 GUI 都能正常运行
- ✅ 安装包大小合理
- ✅ 用户体验良好

---

## 时间估算

| 阶段 | 时间 | 累计 |
|------|------|------|
| 阶段 0：准备 | 0.5h | 0.5h |
| 阶段 1：Workspace | 1h | 1.5h |
| 阶段 2：核心库 | 4h | 5.5h |
| 阶段 3：CLI | 2h | 7.5h |
| 阶段 4：GUI | 2h | 9.5h |
| 阶段 5：清理 | 1h | 10.5h |
| 阶段 6：打包 | 1h | 11.5h |
| **总计** | **11.5h** | - |

## 风险控制

### 每个阶段的检查点
- 编译通过
- 测试通过
- Git 提交
- 可回滚

### 回滚策略
```bash
# 回滚到上一个阶段
git reset --hard HEAD~1

# 回滚到开始
git checkout 1-8-4
```

## 成功标准

- ✅ CLI 独立运行（6MB，~10ms 启动）
- ✅ GUI 独立运行（18MB）
- ✅ 代码复用（核心库）
- ✅ 所有功能正常
- ✅ 所有测试通过

## 下一步

选择一个阶段开始执行，每完成一个阶段提交 Git。
