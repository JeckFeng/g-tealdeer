# tealdeer-tile 统一目录 + 统一搜索 + 统一自定义页面管理（CLI/GUI 同构）技术方案

> 目标：把 tealdeer‑tile 作为“单一产品”，CLI 与 GUI 共用同一套核心数据目录、同一套搜索策略、同一套自定义页面管理能力；彻底消除两套逻辑。当前处于开发阶段，无需兼容旧目录、无需迁移策略。

---

## 0. 方案总览

**核心原则**
- 目录统一：CLI 读取 GUI 的目录（核心功能数据）。
- GUI 私有配置隔离：收藏、主题、布局、窗口状态继续保留在 `tealdeer-tile` 私有目录。
- 搜索统一：将“分词/模糊/回退/过滤”统一下沉至 `tealdeer-core`，CLI 和 GUI 通过 core API 调用。
- 自定义页面管理统一：新增/编辑/追加/禁用/删除统一下沉为 core API。
- 版本号统一：CLI + GUI 全部统一为 **2.0.0**。

**不做的事（当前阶段明确不做）**
- 不做旧目录迁移（开发阶段，不需要兼容）。
- 不做向上游 tealdeer/原版 CLI 兼容说明。

---

## 1. 目录统一设计

### 1.1 统一目录目标
让 CLI 与 GUI 共用以下目录（核心功能数据）：
- cache_dir
- pages_dir
- custom_pages_dir
- shortcut_pages_dir

统一后的实际路径（以当前 GUI 目录为准）：
```
~/.local/share/tealdeer-tile/cache
~/.local/share/tealdeer-tile/pages
~/.local/share/tealdeer-tile/shortcut_pages
```

### 1.2 保留 GUI 私有配置目录
保留在 `~/.local/share/tealdeer-tile` 下的：
- favorites
- theme / layout / immersive settings
- webview 日志
- GUI 运行状态

**这些与 CLI 无关，不对外暴露，不被 CLI 读取。**

### 1.3 修改点（强制统一）
- `tealdeer-core/src/config.rs`
  - **默认路径**不再使用系统 tealdeer 目录，直接指向 `tealdeer-tile`。
  - 删除/忽略旧目录逻辑（当前阶段无兼容）。
- `frontend/tealdeer-widget/src-tauri/src/backend/settings.rs`
  - 继续使用 `APP_DATA_DIR_NAME = "tealdeer-tile"`。
  - 其 `ensure_app_directories` 与 core 目录保持一致。
- CLI 不再使用 OS 默认 `~/.local/share/tealdeer`。

---

## 2. 搜索统一：下沉到 tealdeer-core

### 2.1 统一搜索策略
在 `tealdeer-core` 中实现并公开统一搜索 API，包含：
- **分词策略**：按空格、连字符、下划线分词
- **模糊匹配**：token 子集匹配 + partial includes
- **回退策略**：`systemctl stop` → `systemctl` 回退
- **过滤策略**：渲染时按 token 过滤 example

### 2.2 API 设计（建议）
新增到 `tealdeer-core`（示意）
```rust
pub struct SearchRequest {
  pub query: String,
  pub scope: PageScope,      // Command | Shortcut | All
  pub platform: Vec<PlatformType>,
  pub language: Vec<Language>,
  pub fuzzy: bool,
}

pub struct SearchResult {
  pub name: String,
  pub summary: Option<String>,
  pub scope: PageScope,
  pub source: PageSource,     // tldr/custom/shortcut
}

pub fn search_pages(req: SearchRequest) -> Result<Vec<SearchResult>>;
```

### 2.3 CLI 与 GUI 使用方式
- CLI：新增 `--search`，调用 `core::search_pages`。
- GUI：移除前端 token 子集匹配逻辑，直接调用 core 统一搜索。

---

## 3. 自定义页面管理统一下沉

### 3.1 统一能力范围
将以下操作全部下沉至 `tealdeer-core`：
- create_or_overwrite_page(scope, slug, content)
- append_example(scope, slug, example)
- edit_page(scope, slug)
- edit_patch(scope, slug)
- delete_page(scope, slug)
- enable/disable page
- scan pages

### 3.2 GUI 与 CLI 使用方式
- GUI Tauri 后端不再自行实现文件读写与校验，全部通过 core API。
- CLI 新增：
  - `--shortcut` + `--edit-page` / `--edit-patch`
  - `--shortcut` + `--create/append/delete/enable/disable` 等能力

### 3.3 目录与 scope 绑定
- scope=Command → `custom_pages_dir`
- scope=Shortcut → `shortcut_pages_dir`

---

## 4. CLI 参数扩展

### 4.1 新增参数
- `--search <query>`：命令/快捷键搜索
- `--shortcut`：作用于搜索与自定义页面管理

### 4.2 CLI 命令行为
- `tldr --shortcut --search "neovim close"` → 搜索 shortcut pages
- `tldr --search "systemctl stop"` → core fallback + filter
- `tldr --shortcut neovim close tab` → 渲染该 shortcut page

---

## 5. GUI 改造（调用替换）

### 5.1 前端移除逻辑
- 删除前端 token 子集匹配逻辑
- 删除前端 fallback 判断逻辑（仅显示 core 返回的 result）

### 5.2 后端调用统一
- GUI search 全部调用 `core::search_pages`
- 渲染逻辑只使用 `core::run`

---

## 6. 版本号统一

版本号全部统一为 **2.0.0**，涉及：
- `Cargo.toml`（workspace + tealdeer-core + tldr）
- `frontend/tealdeer-widget/package.json`
- `frontend/tealdeer-widget/src-tauri/tauri.conf.json`
- `frontend/tealdeer-widget/src-tauri/Cargo.toml`

---

## 7. 安装方式统一评估（CLI/GUI 是否要统一）

### 7.1 现状差异
- CLI：通常通过 `cargo install` / distro 包 / AUR 安装，产物为单一可执行文件。
- GUI：通过 Tauri 打包生成 `.deb/.rpm/.AppImage` 等桌面包，包含 WebView 依赖与桌面元数据。

### 7.2 是否可统一安装方式？
**技术上可统一，但不建议强制合并为单一安装方式。**  
原因：CLI 与 GUI 的发布介质、依赖和启动方式本质不同，合并会放大发布复杂度。

### 7.3 可行的统一方式（可选）
1) **统一发行包（单一安装包）**  
   - 在 GUI 包中额外包含 CLI 二进制（并安装到 `PATH`）。  
   - 优点：用户一次安装获得全部功能。  
   - 风险：  
     - 各平台包规范不同（例如 Arch/DEB/RPM），会增加构建脚本复杂度。  
     - GUI 包依赖 WebView，CLI 用户可能不需要，导致包体积变大。  
2) **保持分离，但目录与版本统一**（推荐）  
   - CLI 与 GUI 独立发布，但共享数据目录与版本号。  
   - 优点：发布更简单，用户可按需安装；核心行为仍完全一致。  
   - 风险：需要确保版本号同步与目录统一在代码层强制落实。  

### 7.4 单独安装场景处理
- **仅安装 CLI**：  
  - 仍使用 `~/.local/share/tealdeer-tile/` 作为默认数据目录。  
  - CLI 自动创建目录即可，无需 GUI。  
- **仅安装 GUI**：  
  - GUI 自带目录初始化与配置创建；CLI 缺失不影响功能。  

### 7.5 是否有必要统一安装方式？
**不必强制统一安装方式，也不必强制相同的默认安装目录。**  
推荐策略：**CLI 与 GUI 保持分离安装，但数据/缓存/自定义页面目录统一，版本号统一**。  
只要做到以下三点即可达到“产品一致性”：  
1) 目录统一（核心数据）  
2) 搜索与管理逻辑统一（core API）  
3) 版本号统一  

### 7.6 单独安装的默认路径约定（明确说明）
- **仅安装 CLI**：仍使用 `~/.local/share/tealdeer-tile/` 作为默认数据目录。  
- **仅安装 GUI**：GUI 自带目录初始化与配置创建；CLI 缺失不影响功能。  

---

## 8. 测试清单（必须）

### 7.1 目录统一
- CLI 执行 `tldr --show-paths`，确认目录全部在 `~/.local/share/tealdeer-tile/` 下
- GUI 打开 Settings，显示相同目录

### 7.2 搜索统一
- CLI `--search` 与 GUI Search 返回一致
- `systemctl stop` 在 CLI 与 GUI 均能回退并过滤
- shortcut 搜索支持模糊匹配（neovim close → neovim-close-tab）

### 7.3 自定义页面统一
- GUI 新建 shortcut page，CLI 可直接显示
- CLI append shortcut example，GUI 搜索可显示

### 7.4 回归测试
- CLI 旧命令不受影响（渲染命令页正常）
- GUI 默认搜索/渲染正常

---

## 9. 风险与注意事项（当前阶段最小化）

- 无需迁移，因此可直接覆盖默认目录
- API 扩展时注意 scope 校验，避免误把 command 写入 shortcut_pages
- CLI 新增参数需与 clap 冲突规则确认

---

## 10. 分阶段实施计划（细化）

### 阶段 1：目录统一
**目标**：CLI 与 GUI 共用 `tealdeer-tile` 核心数据目录。  
**步骤**：  
1) 修改 `tealdeer-core` 默认目录解析逻辑：无配置时默认指向 `~/.local/share/tealdeer-tile/`。  
2) 保留 `TEALDEER_CONFIG_DIR` 与 `--config-path` 的覆盖能力。  
3) 确保 GUI `ensure_app_directories` 与 core 默认路径一致。  
**验收标准**：  
- CLI `tldr --show-paths` 输出全部指向 `~/.local/share/tealdeer-tile/`。  
- GUI Settings 显示同样路径。  

### 阶段 2：搜索逻辑下沉
**目标**：CLI 与 GUI 完全共用搜索策略（分词/模糊/fallback/过滤）。  
**步骤**：  
1) 在 core 新增统一搜索 API（SearchRequest/SearchResult）。  
2) 将前端 token 子集匹配逻辑移除，改为调用 core 搜索 API。  
3) CLI 增加 `--search` 参数，调用 core 搜索 API。  
**验收标准**：  
- CLI `--search` 与 GUI Search 返回一致。  
- `systemctl stop` 在 CLI 与 GUI 都能回退并过滤示例。  
- shortcut 模糊搜索一致（`neovim close` → `neovim-close-tab`）。  

### 阶段 3：自定义页面管理下沉
**目标**：CLI/GUI 复用同一套自定义页面 CRUD/append/patch 能力。  
**步骤**：  
1) 在 core 增加 page 管理 API（create/append/edit/delete/enable/disable/scan）。  
2) GUI Tauri 后端改为调用 core API。  
3) CLI 新增/复用参数（`--shortcut` + page 操作能力）。  
**验收标准**：  
- GUI 新建 shortcut page，CLI 可直接读取。  
- CLI append shortcut example，GUI 搜索能显示。  

### 阶段 4：版本号统一
**目标**：CLI/GUI 全部统一为 2.0.0。  
**步骤**：  
1) 修改 workspace 与 crate 版本号。  
2) 修改 Tauri/前端包版本号。  
**验收标准**：  
- `tldr --version` 与 GUI About 显示 2.0.0。  

### 阶段 5：测试与回归
**目标**：确保全功能可用且无回归。  
**步骤**：  
1) 执行 Rust/前端测试清单。  
2) 验证 CLI/GUI 搜索一致性与目录一致性。  
3) 验证 fallback/过滤机制一致。  
**验收标准**：  
- 核心测试全部通过；CLI 与 GUI 关键路径一致且无异常报错。  

---

## 10. 交付物
- 更新后的 core API + CLI 参数 + GUI 后端调用
- 文档：统一目录与统一搜索的使用说明
- 版本号统一为 2.0.0
