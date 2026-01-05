# Tealdeer 可视化常驻工具窗（Linux）技术设计稿（Tauri2）

> 目标：用 **Tauri（Linux）** 开发一个“常驻小窗 + 托盘 + 全局快捷键召唤”的轻量工具，围绕 **tealdeer（tldr 客户端）** 提供可视化搜索、缓存更新、自定义页面（page/patch）生成与管理，同时 **不影响终端中直接使用 `tldr`** 的习惯与行为。

---

## 0.0 技术选型与版本约束（已确定）

### 0.0.1 脚手架创建记录（create-tauri-app）
> 本项目通过 `npm create tauri-app@latest tealdeer-widget` 创建（安装并执行 `create-tauri-app@4.6.2`）。

- **Identifier（Bundle ID）**：`com.xian00.tealdeer-widget`
- **Frontend language**：TypeScript
- **Package manager**：npm
- **UI template**：Vue（Vue 3）
- **UI flavor**：TypeScript

### 0.0.2 关键技术栈（Tech Stack）
- **桌面框架**：Tauri **v2**（由 `create-tauri-app` v4 模板初始化）
- **前端**：Vue 3 + TypeScript + Vite
- **后端**：Rust（Tauri commands）
- **与 tealdeer 的集成方式**：优先调用系统 `tldr`（tealdeer），缺失时回退到 sidecar（详见第 3 章）

### 0.0.3 版本约束（Minimum / Recommended）
- **Rust/Cargo（最低要求）**：**1.85+**  
  - 原因：Tauri v2 依赖树中会出现 **Rust 2024 Edition（`edition="2024"`）** 或标注 **requires Rust 1.85** 的 crate；低于 1.85（例如 1.83）会在解析依赖阶段失败。
- **Rust toolchain 管理方式**：rustup（建议使用 `stable` channel，并对本项目做 override 或 toolchain 文件锁定）
- **Node.js（建议）**：建议使用 **Node.js LTS**（例如 20+）与配套 npm（具体版本以本机环境为准）

### 0.0.4 版本锁定策略（建议）
为避免“本机可编译、换机器/CI 失败”，建议在仓库内显式锁定 Rust toolchain（任选其一）：

**A) 使用 rustup override（推荐给个人开发）**
```bash
cd <project-root>
rustup override set stable
```

**B) 使用 `rust-toolchain.toml`（推荐给团队/CI）**
在项目根目录创建 `rust-toolchain.toml`：
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```
> 如你希望强制最低版本，可将 `channel` 写成 `"1.85.0"`；但需要自行维护升级节奏。

---
## 0. 范围与约束

### 0.1 适用平台
- 仅 **Linux**（优先 KDE Plasma / Wayland，但不强依赖 KDE；对 Wayland 限制采取可降级策略）。

### 0.2 产品形态
- “小型常驻工具窗（pinned utility window）”
- 托盘图标 + 全局快捷键呼出/隐藏
- 不追求“桌面层 widget”，避免 Wayland 合成器差异导致的行为不可控

### 0.3 非目标（Non-goals）
- 不重写 tealdeer 渲染/索引逻辑（优先调用 CLI）
- 不实现 tldr-pages 的完整编辑器（仅面向个人/轻量）
- 不做跨平台（Windows）适配与深度桌面集成（先 Linux MVP）
- 不做在线搜索 tldr-pages 仓库、PR 提交等重度功能

---

## 1. 总体架构

### 1.1 组件划分（逻辑）
1) **UI 前端（WebView）**  
   - Tabs：Search / New / Manage / Settings  
   - 负责输入校验、列表筛选、Markdown 预览、交互逻辑  
2) **Tauri 后端（Rust）**  
   - 负责：检测 tealdeer、调用 CLI、读写文件、扫描目录、管理配置、托盘与快捷键、打开配置文件  
3) **Tealdeer（系统或 sidecar）**  
   - 默认调用系统 `tldr`（tealdeer）  
   - 找不到则回退到应用内打包的 sidecar tealdeer

### 1.2 核心数据流
- Search：UI → Rust → 执行 `tldr --raw` → stdout(Markdown) → UI 渲染
- New（page/patch）：UI → Rust 生成 Markdown → 写入 custom pages dir → UI 提示 → 可一键 Preview effective output
- Manage：Rust 扫描 custom pages dir → 生成索引列表 → UI 展示 → 删除/禁用/启用/打开文件
- Settings：UI 表单 → Rust 写入 tealdeer config（可选）/应用设置 → 生效

---

## 2. 模块划分（可开发粒度）

### 2.1 后端（Rust）模块
- `backend::tealdeer`
  - tealdeer 可执行文件选择（system/sidecar）
  - CLI 参数构造、执行与超时控制
  - `--show-paths` 解析
- `backend::custom_pages`
  - custom pages dir 扫描、解析、索引缓存
  - page/patch 写入、追加示例、禁用/启用、删除
- `backend::config`
  - 应用设置（UI 交互偏好）
  - tealdeer 配置文件读写（TOML）与“打开配置文件”
- `backend::tray_hotkey`
  - 托盘菜单、窗口显示/隐藏、全局快捷键
- `backend::utils`
  - slug 生成、路径安全、文本转义、hash、日志

### 2.2 前端（UI）模块
- `ui/search`
- `ui/new_page`
- `ui/manage`
- `ui/settings`
- `ui/components`（复用：CommandInput、ExamplesEditor、MarkdownPreview、TagChip、ConfirmDialog 等）

---

## 3. tealdeer 调用策略

### 3.1 选择可执行文件（System → Sidecar）
- **System 优先**：通过 `which tldr`/`which tealdeer`/PATH 搜索
- **Sidecar 回退**：Tauri `externalBin` 打包 tealdeer（针对目标三元组）
- 运行时记录：
  - `active_backend.kind = system | sidecar`
  - `active_backend.bin_path`
  - `active_backend.version = tldr --version`（可选）

### 3.2 路径来源：以 `tldr --show-paths` 为准
- 对 system tealdeer：直接执行 `tldr --show-paths`，解析 `Custom pages dir:` 行作为 primary。
- 对 sidecar tealdeer：同样执行 `--show-paths`。
- **Sidecar 默认完全隔离**：cache / custom pages / config 均写入应用私有目录，不与系统 tealdeer 共享。
- UI 明示：
  - “当前处于 Sidecar 模式：自定义页面不会出现在系统终端的 tealdeer 中”

### 3.3 执行方式与安全要求
- **严禁**拼接 shell 字符串执行（避免注入）
- 使用 `std::process::Command`，参数用数组传递
- UI 默认使用 `--raw` 并强制关闭 `--pager`（避免子进程无 TTY 时阻塞）
- 统一设置：
  - 超时（例如 2~5 秒：Search；30 秒：Update）
  - 取消机制（UI 可中断）
  - 捕获 stdout/stderr；非 0 退出码映射为可读错误

---

## 4. Rust 命令封装接口（Tauri commands）

> 建议：所有 command 返回统一结构 `{ ok: bool, data?: T, error?: {...} }`，便于 UI 统一处理。

### 4.1 环境与路径
- `detect_backend() -> BackendInfo`
  - 输出：system 是否存在、sidecar 是否可用、最终选择、版本号等
- `get_show_paths() -> ShowPaths`
  - 解析：config_dir/config_path/cache_dir/pages_dir/custom_pages_dir
- `open_path(path: String) -> ()`
  - 用 `xdg-open` 打开目录/文件（例如 config.toml、custom pages dir）

### 4.2 搜索与渲染
- `render_tldr(command_tokens: Vec<String>, language: Option<String>, platforms: Vec<String>, raw: bool, color: Option<String>, pager: bool, no_auto_update: bool) -> RenderResult`
  - 实际执行：`tldr --raw [--color ...] [--platform ...]* [--language ...] [--no-auto-update] <COMMAND...>`
  - 说明：UI 默认 `raw=true`，并忽略/禁用 `pager` 以避免阻塞
- `preview_effective_output(command_tokens: Vec<String>, ...) -> RenderResult`
  - 与 `render_tldr` 同源；用于 New/Manage 的“最终效果预览”按钮
- `update_cache(language: Option<String>, quiet: bool) -> UpdateResult`
  - 执行：`tldr --update`（必要时附加 `--language`）

### 4.3 自定义页写入
- `create_or_overwrite_page(req: NewPageRequest) -> CustomFileInfo`
  - 写入：`<slug>.page.md`
- `create_or_overwrite_patch(req: NewPatchRequest) -> CustomFileInfo`
  - 写入：`<slug>.patch.md`
- `append_example_to_page(command: String, example: Example) -> CustomFileInfo`
  - 读取现有 `.page.md`，在末尾追加规范块；若不存在可提示“先创建 page”或自动创建（可配置）

### 4.4 自定义页管理
- `scan_custom_pages() -> Vec<CustomEntry>`
  - 扫描 custom pages dir
  - 识别：page/patch/enabled/disabled
  - 提取：command_slug、类型、路径、mtime、(可选)首行 summary
- `delete_custom_file(path: String) -> ()`
- `disable_custom_file(path: String) -> CustomFileInfo`
- `enable_custom_file(path: String) -> CustomFileInfo`
- `read_custom_file(path: String) -> String`
- `write_custom_file(path: String, content: String) -> ()`（可选，提供“高级编辑”）

### 4.5 设置
- `get_app_settings() -> AppSettings`
- `set_app_settings(settings: AppSettings) -> ()`
- `get_tealdeer_config() -> String`（读取 config.toml 原文，便于 UI 展示）
- `set_tealdeer_config(patch: TealdeerConfigPatch) -> ()`
  - 只写你承诺支持的字段（language/platform/auto_update/pager/color/archive_source 等）
- `open_tealdeer_config() -> ()`（打开 config.toml）

---

## 5. 目录扫描与索引结构

### 5.1 需要扫描的目录
- `custom_pages_dir`（来自 `tldr --show-paths`）

### 5.2 文件匹配规则
- **Enabled**
  - `*.page.md`
  - `*.patch.md`
- **Disabled（建议策略：后缀加 .disabled）**
  - `*.page.md.disabled`
  - `*.patch.md.disabled`

> 禁用策略必须“单一且确定”，避免用户随意改名导致不可恢复。

### 5.3 索引条目结构（Rust 内部模型）
```json
{
  "command_slug": "git-commit",
  "display_command": "git commit",
  "kind": "page|patch",
  "status": "enabled|disabled",
  "path": "/home/.../git-commit.page.md",
  "mtime": 1700000000,
  "size": 1234,
  "summary": "optional first block quote line",
  "examples_count": 4
}
```

### 5.4 生成索引的策略
- **MVP**：每次打开 Manage 页面时扫描目录（成本小、实现简单）
- **优化**（可选）：缓存索引到 `app_data/index.json`
  - 再次扫描时只更新 changed files（对比 mtime/size/hash）
- **文件解析**（轻量）：
  - `summary`：取第一条 `>` 的内容
  - `examples_count`：统计以 `- ` 开头的段落数量（近似即可）

---

## 6. 文件命名与命令 slug 规则

### 6.1 目标
- 用户输入可能为：`git log`、`systemctl status`、`pacman`
- 文件名必须：安全、可逆（尽量）、避免路径穿越、跨文件系统兼容

### 6.2 slug 生成规则（建议）
- **安全校验（先于 slug）**：若命令包含路径分隔符（`/`、`\`）或路径穿越片段（`..`），直接拒绝
- trim 两端空白
- 连续空白（空格/Tab）→ 单个 `-`
- 允许字符集：`[a-zA-Z0-9._+-]`
- 其他字符（如 `/`、`:`、`|`、中文）→ `-`
- 连续 `-` 压缩为单个 `-`
- 最终若为空：拒绝

示例：
- `git log` → `git-log`
- `pacman` → `pacman`
- `docker compose` → `docker-compose`
- `rg "a|b"` → `rg-a-b`（仅文件名；显示命令仍保留原始输入）

### 6.3 文件名映射
- page：`<slug>.page.md`
- patch：`<slug>.patch.md`
- disabled：在末尾追加 `.disabled`

---

## 7. 自定义页生成：Markdown 规范与写入策略

### 7.1 页面模板（Custom Page）
```md
# {{Command}}
> {{Summary}}

- {{Example 1 description}}:
`{{Example 1 command}}`

- {{Example 2 description}}:
`{{Example 2 command}}`
```

### 7.2 补丁模板（Patch）
> Patch 会“追加到已有页面末尾”，为避免重复标题，建议默认不写 `#` 标题（可做成开关）。

推荐默认：
```md
- {{Example description}}:
`{{Example command}}`
```

可选（用户勾选“Include header”）：
```md
# {{Command}} (custom patch)

- {{Example description}}:
`{{Example command}}`
```

### 7.3 Append Example 到现有 Custom Page 的策略
- 读取 `.page.md` 原文
- 在文件末尾追加一个空行 + 新示例块（保持与现有格式一致）
- 若不存在 `.page.md`：
  - 默认：提示用户先创建 Custom Page
  - 可选设置：自动创建一个最小 page（仅 summary + 新示例）

### 7.4 占位符（可选增强）
- UI 提供“插入占位符”按钮：`{{package}}`、`{{path}}`、`{{port}}`
- 不强制；仅提升规范性与可读性

---

## 8. 禁用/启用/删除策略

### 8.1 删除
- 直接删除文件（`rm` 等价）
- 风险提示：不可恢复（可选：先移到回收站目录 `pages.trash/`，默认关闭以保持轻量）

### 8.2 禁用（推荐：后缀 `.disabled`）
- enabled → disabled：`foo.page.md` → `foo.page.md.disabled`
- disabled → enabled：反向改名
- 需要处理重名冲突：
  - 若目标已存在，拒绝并提示（或加时间戳后缀）

### 8.3 与终端共存（关键承诺）
- System 模式：写入系统 tealdeer 的 custom pages dir → 终端立即生效
- Sidecar 模式：写入 sidecar 的 custom pages dir（或应用私有目录）→ 终端不生效，UI 必须醒目标注

---

## 9. UI 线框图（Wireframe）

> 目标：一个小而克制的窗口，提供 4 个核心面板。

### 9.1 主窗口（Pinned Utility Window）
```
┌──────────────────────────────────────────────┐
│ TLDR Widget      [Search] [New] [Manage] [⚙] │
├──────────────────────────────────────────────┤
│  Search Panel                                  │
│  Command: [ pacman____________________ ] [Run] │
│  Language: [zh v]  Platform: [linux v] [Raw□]  │
│  Output:                                         │
│  ┌──────────────────────────────────────────┐  │
│  │ (rendered tldr content / markdown)       │  │
│  │ ...                                      │  │
│  └──────────────────────────────────────────┘  │
│  [Copy] [Preview Raw] [Update Cache]            │
└──────────────────────────────────────────────┘
```

### 9.2 New Panel（创建/覆盖/补丁/追加示例）
```
Mode: ( ) Custom Page  ( ) Patch  ( ) Append Example
Command: [ git log__________________________ ]
Summary: [ one-line summary_________________ ]
Examples:
  [ + Add Example ]
  ┌ desc: [...............]  cmd: [............... ] ┐
  └ desc: [...............]  cmd: [............... ] ┘
[Generate] [Preview Effective Output] [Open Custom Dir]
```

### 9.3 Manage Panel（列表/筛选/搜索/操作）
```
Filter: Type [All|Page|Patch]  Status [All|Enabled|Disabled]
Search: [ fuzzy query________________ ]
┌─────────────────────────────────────────────────────────┐
│ [E] pacman      Page   Enabled   mtime ...  [Open][..]  │
│ [D] git-log     Patch  Disabled  mtime ...  [Enable]    │
│ ...                                                     │
└─────────────────────────────────────────────────────────┘
Actions: [Delete] [Disable/Enable] [Edit] [Preview Effective]
```

### 9.4 Settings Panel（受控表单 + 打开配置文件）
```
Defaults:
  Language: [zh v]   Platform order: [linux, common v]
Update:
  Auto update: [on/off]   Interval(hours): [24]
Display:
  Pager: [on/off]   Color: [auto/always/never v]
Advanced:
  Archive source: [https://... ]  TLS backend: [default/native-tls v]
Buttons: [Save] [Open config.toml] [Show paths]
```

### 9.5 托盘菜单（Tray）
```
TLDR Widget
- Show/Hide (Global hotkey: Ctrl+Alt+T)
- Search...
- New Custom Page...
- Update cache
- Open custom pages dir
- Settings
- Quit
```

---

## 10. 设置与配置：应用设置 vs tealdeer 配置

### 10.1 两类配置的边界
- **AppSettings（应用设置）**：窗口行为、UI 默认选项、快捷键、主题等
- **Tealdeer config.toml**：language/platform/auto_update/pager/color/archive_source 等

建议策略：
- UI 只对 tealdeer 的“关键少量字段”做表单写入
- 其余字段：提供“打开配置文件”按钮，用户自行编辑

### 10.2 写入策略（避免破坏用户原配置）
- System 模式：默认 **只读** tealdeer config（除非用户明确启用“由本应用管理 tealdeer 配置”）
- Sidecar 模式：默认全权管理（放在 app_data 的 config.toml），并强制使用该路径运行 tealdeer（例如 `--config-path` 或 `TEALDEER_CONFIG_DIR`）

---

## 11. 兼容性与降级策略（Wayland/桌面环境差异）

- 置顶（always-on-top）不可用或不稳定时：
  - 允许用户关闭置顶
  - 可选：失焦自动隐藏（更符合工具窗）
- 透明度/圆角等装饰：
  - 以“可选主题能力”实现，无法保证所有合成器一致
- 托盘不可用时：
  - 保留全局快捷键
  - 提供命令行参数 `--show`/`--hide`（可选）

---

## 12. 打包与部署（Linux）

### 12.1 Sidecar tealdeer
- 使用 Tauri `bundle.externalBin` 打包 tealdeer 二进制
- 构建时按目标三元组输出不同文件名后缀

### 12.2 权限与目录
- 应用私有目录：`$XDG_DATA_HOME/<app>/`、`$XDG_CONFIG_HOME/<app>/`
- 不需要 root 权限
- 不修改系统文件

---

## 13. 测试计划（建议最低集）

### 13.1 单元测试（Rust）
- slug 生成（多单词、特殊字符、空输入）
- Markdown 生成（page/patch/append）
- 扫描识别（enabled/disabled/page/patch）
- 禁用/启用重命名冲突处理

### 13.2 集成测试
- 在 CI 容器中运行 sidecar tealdeer：
  - `--show-paths` 可解析
  - `tldr tar` 输出非空
  - 写入自定义页后 `tldr <cmd>` 可见变化（page 覆盖）

### 13.3 手动验收（桌面）
- KDE Plasma/Wayland：快捷键呼出隐藏、托盘、窗口置顶/透明度可用性
- 终端一致性：系统 tealdeer 模式下，新增 page/patch 终端立刻可见

---

## 14. MVP 里程碑拆解（建议）

1) Search（运行 tealdeer + 渲染输出）
2) Backend detection + show-paths + open custom dir
3) New Custom Page（.page.md）+ Preview effective output
4) Manage（扫描 + 删除/禁用/启用）
5) Patch（.patch.md）+ 预览最终输出
6) Settings（受控字段写入 + open config）
7) Tray + Global hotkey + Window show/hide

---

## 15. 关键实现细节（工程注意点）

- 所有外部命令执行必须：参数数组 + 超时 + stderr 捕获
- 文件操作必须：路径归一化，限制只在 custom_pages_dir 内操作（防止任意路径删除）
- Command 输入安全校验：包含 `/`、`\` 或 `..` 直接拒绝（写文件场景）
- UI 输入的 `Command` 与 `Examples.command` 应做最小校验（非空、长度限制），但不做“命令合法性”判断
- Preview 采用实际 `tldr --raw <cmd>` 输出，避免自行合并导致不一致
- Update 缓存失败时：显示 stderr，并引导用户检查代理/配置（尤其 archive_source）

---

## 附录 A：数据结构（建议）

### A.1 NewPageRequest
```json
{
  "command": "pacman",
  "summary": "Arch Linux package manager utility.",
  "examples": [
    {"desc": "Synchronize and update all packages", "cmd": "sudo pacman -Syu"},
    {"desc": "Install a new package", "cmd": "sudo pacman -S {{package}}"}
  ],
  "mode": "page|patch|append",
  "include_header_in_patch": false
}
```

### A.2 AppSettings（示例）
```json
{
  "default_language": "zh",
  "default_platforms": ["linux", "common"],
  "allow_update_from_ui": true,
  "use_pager": false,
  "color": "auto",
  "hotkey_toggle": "Ctrl+Alt+T",
  "window": {
    "always_on_top": true,
    "opacity": 0.95,
    "rounded": 12,
    "border": true
  }
}
```

---

## 附录 B：实现提示（可选技术栈）
- 前端：Svelte/React 均可；优先选你熟悉的以缩短周期
- Markdown 渲染：前端使用轻量 renderer（支持 code block、list、blockquote）
- 搜索：Manage 列表用 `fuzzy match`（例如 `fuse.js`）即可

---

**文件结束**  
