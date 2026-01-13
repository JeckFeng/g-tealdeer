# Shortcut Keys 终极技术方案（Final）

## 1. 两份方案对比结论

### 1.1 综合评价结论
**以 Codex 方案为主骨架（核心架构、CLI、目录隔离）+ 吸收 Kiro 的 UX 改进（统一搜索分组展示、快捷键高亮渲染）。**

原因：Codex 方案更贴近现有 tealdeer-core 的运行方式，改动路径清晰、稳定性更高；Kiro 方案在 UX 上更先进，但引入“统一搜索引擎 + 大量新模块”的重构成本更高、风险更大。最终方案在保持低风险架构的前提下，补齐统一搜索与交互体验。

### 1.2 维度对比（结论摘要）
| 维度 | Codex | Kiro | 结论 |
|---|---|---|---|
| 实施难度 | 中 | 中偏高 | Codex 更稳 | 
| 需求理解 | 正确且聚焦 | 正确且更深入 UX | Kiro 略优，但更重 |
| 技术选型 | 复用 tealdeer-core | 新建 search_engine | Codex 更保守可靠 |
| 架构合理性 | 与现有架构一致 | 改动面更大 | Codex 更可控 |
| 维护性 | 高 | 中 | Codex 更易维护 |

**最终结论：** 采用 Codex 的核心/目录/CLI 设计，同时引入 Kiro 的“统一搜索 + 分类展示”作为 UI 层增强。

---

## 2. 功能需求细化（Final）
### 2.1 必需功能
- 快捷键页支持 CRUD（新建/追加/禁用/启用/删除/编辑）。
- 快捷键页可搜索、可收藏、可打开文件。
- CLI 支持查询/管理快捷键页。
- 快捷键页与 TLDR 缓存、命令自定义页 **完全隔离**。

### 2.2 Markdown 规范
保持 TLDR 自定义 page 格式不变：
```
# app-name
> summary
- desc:
`shortcut`
```

### 2.3 收藏规范
收藏项必须包含：
- 快捷键组合（cmd）
- 功能描述（desc）
- 页面类型（command / shortcut）

---

## 3. 模块划分（Final）
### 3.1 tealdeer-core（Rust）
新增并最小化改动：
- `Config.directories.shortcut_pages`
- `RunArgs` 新增 `scope: PageScope`（`command` / `shortcut`）
- `CacheConfig` 支持切换目录  
  - command：现有缓存 + custom_pages_dir  
  - shortcut：shortcut_pages（与 custom_pages_dir 类似逻辑）
- `show-paths` 输出 shortcut_pages
- CLI 新增 `--shortcut`（短参 `-s`）

### 3.2 Tauri 后端
推荐 **复用现有 custom_pages 逻辑**，抽象为泛型页面管理器：
- `page_manager.rs`：核心文件读写、禁用/启用、扫描、追加 patch
- `custom_pages.rs` 与 `shortcut_pages.rs` 共享同一套实现
- `render_tldr` 增加 `scope` 参数（或新增 `render_shortcut_tldr`）

### 3.3 前端 UI
新增页面类型状态 `pageType`：
- Search：默认 `All`，可切换 `Command` / `Shortcut`
- New Page：强制选择类型
- Manage：默认 `All`，支持标签过滤
- Favorites：按类型分组展示

---

## 4. 技术策略与架构细节
### 4.1 目录结构
```
app_data_dir/
  cache/                 # TLDR 缓存
  pages/                 # 自定义命令页
  shortcut_pages/        # 快捷键页（新增）
```

### 4.2 CLI 扩展
推荐策略：
- `tldr <name>`（默认 command 行为）
- `tldr --shortcut <name>` 或 `tldr -s <name>`
- `tldr --list --shortcut`
- `--edit-page/--edit-patch` 在 `--shortcut` 下打开快捷键文件

### 4.3 搜索策略
结合 Codex 架构 + Kiro UX：
- **All 模式（UI）**：先查询 command，再查询 shortcut，结果分组展示
- **Command/Shortcut 模式**：只查询指定类型
- CLI 使用 `--shortcut` 显式选择快捷键页，默认 command 行为不变

### 4.4 收藏结构（升级版）
推荐 key 结构：
```json
{
  "version": 2,
  "items": {
    "command::nano": [{ "command": "nano file.txt", "description": "Open file" }],
    "shortcut::nano": [{ "command": "Ctrl + O", "description": "保存当前内容" }]
  }
}
```

---

## 5. UI/UX 设计（Final）
### 5.1 Search 页面
```
[All | Command | Shortcut Keys]
输入框统一，结果分组显示：
📦 Commands (x)
⌨️ Shortcut Keys (y)
```
优势：既满足“区分类型”的约束，也减少用户切换成本。

### 5.2 New Page 页面
```
Page Type:
○ Command Page
● Shortcut Keys Page
```
选择后动态切换表单提示，但字段结构可复用。

### 5.3 Manage 页面
- 列表项标签区分 `Command / Shortcut / Patch`
- 顶部过滤器：`All | Command | Shortcut | Patch`

### 5.4 渲染优化
- Shortcut Keys 使用专门 tokenizer 高亮（Ctrl/Alt/Shift 等）
- 支持复制/收藏单条快捷键

---

## 6. 复杂度与工作量评估（Final）
### 6.1 改造难度
中等偏上（涉及 core + Tauri + 前端 UI）。

### 6.2 工期估算（单人）
- Core/CLI/config：1.5 ~ 2 天
- Tauri 后端：1 ~ 1.5 天
- UI 改造：1.5 ~ 2 天
- 测试与修复：1 天

**总计：5 ~ 7 天**

---

## 7. 风险与对策
- **同名冲突**：favorites 使用 `page_type::title` 作为 key。
- **兼容性**：默认 command 行为不变，旧用法无影响。
- **目录升级**：首次启动自动创建 `shortcut_pages`。

---

## 8. 实施步骤（建议）
1. core 配置与 CLI shortcut
2. Tauri shortcut 管理接口
3. UI 类型切换与展示
4. 收藏结构升级
5. 测试与回归

---

## 9. 具体接口清单（Final）
### 9.1 tealdeer-core CLI / API
**新增类型：**
```rust
enum PageScope {
  Command,
  Shortcut,
}
```

**RunArgs 扩展：**
```rust
pub struct RunArgs {
  pub scope: PageScope,      // 默认 Command
  // 现有字段保持不变
}
```

**CLI 参数（最终选择）：**
- `--shortcut`（短参 `-s`）

**行为约定：**
- 未指定 `--shortcut`：现有 command 行为不变
- 指定 `--shortcut`：仅查找 `shortcut_pages` 下的 `.page.md/.patch.md`

### 9.2 tealdeer-core 配置
**Config 新字段：**
```toml
[directories]
shortcut_pages = "/path/to/shortcut_pages"
```

**show-paths 增强：**
```
Shortcut pages dir: /.../shortcut_pages
```

### 9.3 Tauri IPC 接口（新增或改造）
**渲染接口（推荐统一）：**
```ts
render_tldr({
  commandTokens: string[],
  scope: "command" | "shortcut",
  language?: string,
  platforms?: string[],
  raw?: boolean,
  color?: string,
  pager?: boolean,
  noAutoUpdate?: boolean,
})
```

**快捷键页管理（推荐独立接口，内部复用通用 manager）：**
```ts
create_or_overwrite_shortcut_page(req: NewPageRequest)
create_or_overwrite_shortcut_patch(req: NewPatchRequest)
append_example_to_shortcut_page(command: string, example: ExampleInput)
scan_shortcut_pages() -> CustomEntry[]
enable_shortcut_file(path: string)
disable_shortcut_file(path: string)
delete_shortcut_file(path: string)
read_shortcut_file(path: string) -> string
open_shortcut_pages()
open_shortcut_page_file(path: string)
```

**搜索候选列表（支持 UI 分组展示）：**
```ts
search_pages({
  query: string,
  scope: "command" | "shortcut" | "all"
}) -> Array<{
  name: string,
  summary?: string,
  scope: "command" | "shortcut",
  source: "tldr" | "custom" | "shortcut"
}>
```
实现建议：基于 `Cache::list_pages` + 本地文件摘要提取。

### 9.4 前端状态与接口
- `searchScope: "all" | "command" | "shortcut"`
- `newPageType: "command" | "shortcut"`
- `manageFilterType: "all" | "command" | "shortcut" | "patch"`
- `favorites` key 规则升级为 `"{scope}::{title}"`

---

## 10. 改造任务列表（拆分到文件级）
### 10.1 tealdeer-core
- `tealdeer-core/src/cli.rs`：新增 `--shortcut`（`-s`）参数。
- `tealdeer-core/src/api.rs`：`RunArgs` 增加 `scope`，并在 `run_inner` 中切换目录策略。
- `tealdeer-core/src/config.rs`：新增 `shortcut_pages` 字段并提供默认路径。
- `tealdeer-core/src/cache.rs`：确保 `scope=shortcut` 仅基于 custom/shortcut dir 查找。

### 10.2 Tauri 后端
- `frontend/tealdeer-widget/src-tauri/src/backend/tealdeer.rs`：`render_tldr` 增加 `scope` 参数。
- `frontend/tealdeer-widget/src-tauri/src/backend/settings.rs`：确保 app_data_dir 下创建 `shortcut_pages`，并写入 config。
- `frontend/tealdeer-widget/src-tauri/src/backend/custom_pages.rs`：抽象为 `page_manager.rs`，支持传入目录路径。
- 新增 `frontend/tealdeer-widget/src-tauri/src/backend/shortcut_pages.rs`。
- `frontend/tealdeer-widget/src-tauri/src/backend/open_paths.rs`：支持打开 shortcut 目录与文件。
- `frontend/tealdeer-widget/src-tauri/src/lib.rs`：注册新增 IPC。

### 10.3 前端 UI
- `frontend/tealdeer-widget/src/App.vue`：
  - Search / New / Manage 增加 `scope` 选择
  - Search 支持 `All` 分组展示（调用 `search_pages`）
  - Favorites 按 `scope::title` 归档
- `frontend/tealdeer-widget/src/components/*`：
  - 新增 `PageTypeSelector.vue`（或复用现有控件）
  - 新增 `SearchResultGroup.vue`
  - 新增 `ShortcutKeyLine.vue`（快捷键高亮与复制）
- `frontend/tealdeer-widget/src/utils/shortcut_tokenizer.ts`：快捷键解析。
- `frontend/tealdeer-widget/src/locales/*.json`：增加新翻译文案。

### 10.4 数据迁移与兼容
- favorites.json 从 `version=1` 升级到 `version=2`，无损迁移。
- CLI 默认 command 行为不变。

---

## 11. 测试用例清单（核心 + UI）
### 11.1 tealdeer-core 单元测试
- `scope=command` 仍可查询 TLDR cache。
- `scope=shortcut` 只读 `shortcut_pages`，不返回 TLDR pages。
- `list` 在 `scope=shortcut` 返回 `.page.md` 列表。
- `--edit-page` / `--edit-patch` 在 `--shortcut` 模式打开 shortcut 文件。

### 11.2 Tauri 后端测试
- `create_or_overwrite_shortcut_page` 写入正确路径与内容。
- `append_example_to_shortcut_page` 格式正确。
- `scan_shortcut_pages` 正确识别 enabled/disabled。
- `render_tldr(scope=shortcut)` 输出成功。

### 11.3 前端手工测试（关键路径）
- Search / All：输入 `vim`，显示 command + shortcut 分组。
- Search / Shortcut：只显示快捷键结果。
- New Page / Shortcut：创建并预览渲染正常。
- Manage：标签与过滤器正确。
- Favorites：收藏快捷键后显示在 Settings 的 Shortcut 分组。
- 打开文件：快捷键页可直接打开。

### 11.4 边界与回归
- 快捷键包含特殊字符（`Ctrl+[`、`Alt+Shift+F12`）。
- 同名页面 `command::nano` 与 `shortcut::nano` 同时存在。
- 升级后旧收藏仍可读取。

---

## 12. 分阶段分步骤改造计划（详细）
### 阶段 0：准备与对齐（0.5 天）
1. 确认 CLI 参数最终形式（`--shortcut` / `-s`）。
2. 确认目录命名 `shortcut_pages`。
3. 对齐 UI 方案（All/Command/Shortcut 三态）。

### 阶段 1：Core & Config（1.5~2 天）
1. `config.rs`：新增 `shortcut_pages` 默认路径。
2. `cli.rs`：新增 CLI 参数，映射到 `RunArgs.scope`。
3. `api.rs`：根据 scope 构造 `CacheConfig`。
4. `cache.rs`：保证 scope=shortcut 只匹配 custom pages。
5. 更新 `show-paths` 输出。

### 阶段 2：Tauri 后端（1~1.5 天）
1. 抽象 `page_manager.rs`（封装 CRUD + scan）。
2. 新增 `shortcut_pages.rs`，复用 manager。
3. `tealdeer.rs` 的 render API 增加 scope。
4. `settings.rs` 创建 shortcut 目录。
5. `open_paths.rs` 新增 shortcut 目录/文件打开函数。
6. `lib.rs` 注册 IPC。

### 阶段 3：前端 UI（1.5~2 天）
1. Search：加入 page type selector + 结果分组展示。
2. New Page：加入类型选择与提示文案。
3. Manage：加入 type 标签与过滤。
4. Favorites：升级为 `scope::title` 分组。
5. 组件新增：快捷键高亮行渲染。

### 阶段 4：收藏升级（0.5~1 天）
1. favorites.json schema 升级。
2. UI 按 scope 分组展示。
3. 兼容旧数据读取。

### 阶段 5：测试与回归（1 天）
1. 单元测试覆盖 scope。
2. UI 关键路径测试。
3. 兼容与回归测试。

### 阶段 6：文档与交付（0.5 天）
1. 更新 README/帮助文档。
2. 新增快捷键示例模板。
