# 阶段 1：Core & Config 完成报告

**项目:** Tealdeer-Tile Shortcut Keys Extension  
**阶段:** 1 - Core & Config  
**日期:** 2026-01-12  
**状态:** ✅ 完成

---

## 实施概述

按照 `Shortcut_Keys_Plan_Final.md` 文档，完成了阶段 1 的所有任务，实现了核心配置和 CLI 参数支持。

---

## 完成的任务

### 1. config.rs - 配置文件支持

**修改文件:** `tealdeer-core/src/config.rs`

**变更内容:**
- `RawDirectoriesConfig` 添加 `shortcut_pages_dir: Option<PathBuf>` 字段
- `DirectoriesConfig` 添加 `shortcut_pages_dir: Option<PathWithSource>` 字段
- `Config::load` 中添加 shortcut_pages_dir 初始化逻辑
- 默认路径：`{app_data}/shortcut_pages/`（与 custom_pages 平行）

**代码行数:** +18 行

---

### 2. types.rs - PageScope 枚举

**修改文件:** `tealdeer-core/src/types.rs`

**变更内容:**
```rust
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageScope {
    Command,
    Shortcut,
}

impl Default for PageScope {
    fn default() -> Self {
        Self::Command
    }
}
```

**代码行数:** +13 行

---

### 3. cli.rs - CLI 参数

**修改文件:** `tealdeer-core/src/cli.rs`

**变更内容:**
```rust
/// Search for shortcut keys instead of commands
#[arg(short = 's', long = "shortcut")]
pub shortcut: bool,
```

**代码行数:** +4 行

---

### 4. api.rs - 核心逻辑

**修改文件:** `tealdeer-core/src/api.rs`

**变更内容:**

1. **RunArgs 添加 scope 字段:**
```rust
pub struct RunArgs {
    pub scope: crate::types::PageScope,
    pub command: Vec<String>,
    // ...
}
```

2. **ShowPaths 添加 shortcut_pages_dir:**
```rust
pub struct ShowPaths {
    // ...
    pub shortcut_pages_dir: Option<String>,
    pub custom_pages_dir: Option<String>,
}
```

3. **run_inner 中根据 scope 切换目录:**
```rust
let custom_pages_dir = match args.scope {
    crate::types::PageScope::Command => config.directories.custom_pages_dir.as_ref(),
    crate::types::PageScope::Shortcut => config.directories.shortcut_pages_dir.as_ref(),
}
.map(PathWithSource::path);
```

4. **cache_config 根据 scope 设置 pages_directory:**
```rust
let dummy_pages_dir = config.directories.cache_dir.path().join("__shortcut_only__");
let pages_directory = match args.scope {
    crate::types::PageScope::Command => config.directories.cache_dir.path().join(TLDR_PAGES_DIR),
    crate::types::PageScope::Shortcut => {
        let _ = std::fs::create_dir_all(&dummy_pages_dir);
        dummy_pages_dir
    }
};
```

5. **format_show_paths 添加 shortcut_pages_dir 输出:**
```rust
let shortcut_pages_dir_display = match config.directories.shortcut_pages_dir {
    Some(ref path_with_source) => path_with_source.to_string(),
    None => "[None]".to_string(),
};
```

**代码行数:** +35 行

---

### 5. main.rs - 参数映射

**修改文件:** 
- `tealdeer-core/src/main.rs`
- `tldr/src/main.rs`

**变更内容:**
```rust
use tealdeer::{api::RunArgs, run, types::PageScope, Cli};

let run_args = RunArgs {
    command: args.command,
    scope: if args.shortcut { PageScope::Shortcut } else { PageScope::Command },
    // ...
};
```

**代码行数:** +4 行（每个文件 +2 行）

---

### 6. Tauri 后端兼容

**修改文件:** `frontend/tealdeer-widget/src-tauri/src/backend/tealdeer.rs`

**变更内容:**
- 导入 `PageScope`
- 所有 `RunArgs` 初始化添加 `scope: PageScope::Command`

**代码行数:** +4 行

---

## 测试结果

### 编译测试 ✅

```bash
$ cargo check
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.57s

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 33.60s
```

**结果:** 无错误，无警告

---

### 功能测试 ✅

#### 1. CLI 参数显示

```bash
$ tldr --help | grep shortcut
  -s, --shortcut             Search for shortcut keys instead of commands
```

**结果:** ✅ 参数正确显示

---

#### 2. show-paths 输出

```bash
$ tldr --show-paths
Config dir:       /home/xian00/.config/tealdeer/ (OS convention)
Config path:      /home/xian00/.config/tealdeer/config.toml (OS convention)
Cache dir:        /home/xian00/.cache/tealdeer (OS convention)
Pages dir:        /home/xian00/.cache/tealdeer/tldr-pages/
Shortcut pages dir: /home/xian00/.local/share/tealdeer/shortcut_pages/ (OS convention)
Custom pages dir: /home/xian00/.local/share/tealdeer/pages/ (OS convention)
```

**结果:** ✅ shortcut_pages_dir 正确显示

---

#### 3. 快捷键页面渲染

**测试文件:** `~/.local/share/tealdeer/shortcut_pages/vim.page.md`

```bash
$ tldr --shortcut vim

  Vim 编辑器快捷键参考

  保存文件:

      Ctrl + O

  退出编辑器:

      Ctrl + X

  剪切当前行:

      Ctrl + K

  粘贴:

      Ctrl + U
```

**结果:** ✅ 快捷键页面正确渲染

---

#### 4. 快捷键列表

```bash
$ tldr --shortcut --list
vim
```

**结果:** ✅ 列出快捷键页面

---

#### 5. 隔离性测试

```bash
# 测试 scope=shortcut 不查找 TLDR cache
$ tldr --shortcut tar
Page `tar` not found in cache.

# 测试 scope=command 正常工作
$ tldr tar
  归档实用程序。
  通常与压缩方法结合使用，例如 `gzip` 或 `bzip2`。
  ...
```

**结果:** ✅ 完全隔离，互不影响

---

## 代码统计

| 文件 | 新增行数 | 修改行数 |
|------|---------|---------|
| tealdeer-core/src/config.rs | 18 | 0 |
| tealdeer-core/src/types.rs | 13 | 0 |
| tealdeer-core/src/cli.rs | 4 | 0 |
| tealdeer-core/src/api.rs | 35 | 5 |
| tealdeer-core/src/main.rs | 2 | 1 |
| tldr/src/main.rs | 2 | 1 |
| frontend/.../tealdeer.rs | 4 | 3 |
| **总计** | **78** | **10** |

---

## 技术亮点

### 1. 完全隔离的目录结构

- `shortcut_pages/` 与 `pages/` 平行，互不干扰
- scope=shortcut 时使用 dummy pages_directory，不查找 TLDR cache
- 保证了快捷键和命令的完全隔离

### 2. 向后兼容

- 默认行为不变（scope=Command）
- 现有代码无需修改
- 配置文件可选（使用默认路径）

### 3. 类型安全

- 使用 PageScope 枚举而非字符串
- 编译时检查，避免运行时错误
- 实现 Default trait，简化初始化

### 4. 最小化改动

- 复用现有 custom_pages 逻辑
- 只在必要的地方添加 scope 判断
- 代码改动集中，易于维护

---

## 遇到的问题与解决

### 问题 1: Cache::open 检查 pages_directory 存在性

**现象:** scope=shortcut 时，Cache::open 返回 None，因为 dummy pages_directory 不存在

**解决:** 在设置 pages_directory 时自动创建 dummy 目录
```rust
let _ = std::fs::create_dir_all(&dummy_pages_dir);
```

---

### 问题 2: cache_config 使用错误的 custom_pages_directory

**现象:** 快捷键页面找不到，因为 cache_config 仍使用 custom_pages_dir 而非 shortcut_pages_dir

**解决:** 使用之前计算的 custom_pages_dir 变量
```rust
custom_pages_directory: custom_pages_dir,  // 而非 config.directories.custom_pages_dir
```

---

## 下一步

阶段 1 已完成，可以继续：

- **阶段 2:** Tauri 后端（shortcut_pages 管理接口）
- **阶段 3:** 前端 UI（类型切换与展示）
- **阶段 4:** 收藏结构升级
- **阶段 5:** 测试与回归

---

**完成时间:** 2026-01-12 23:45  
**总耗时:** 约 1.5 小时  
**状态:** ✅ 所有任务完成，所有测试通过

