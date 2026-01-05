# Tauri2 开发计划（Tealdeer 可视化常驻工具窗 · Linux）

本文基于《Tealdeer 可视化常驻工具窗（Linux）技术设计稿（Tauri2）》制定，按阶段交付可运行的 MVP。默认 UI 使用 `tldr --raw`，Sidecar 完全隔离（cache/custom pages/config 独立）。

---

## 阶段 0：基线与工程约束（1–2 天）
**目标**：建立可重复的构建环境与最小运行链路。  
**任务**：
- 在仓库根记录 Rust/Node 版本约束，添加 `rust-toolchain.toml`（stable/1.85+）。
- 在 `/mnt/data_nvme/code/tealdeer/frontend/tealdeer-widget` 内确认 Tauri v2 项目结构，补充 README 的本地启动步骤。
- 明确 Linux 发行版、桌面环境与 Wayland/X11 兼容范围，记录已知限制。
**交付物**：
- `rust-toolchain.toml`、更新后的 README。
- 能 `npm install` + `npm run dev` 启动 UI（空页面即可）。
**验收**：本机可编译 Tauri 项目并显示默认页面。

---

## 阶段 1：后端执行器与安全边界（2–4 天）
**目标**：Rust 后端可安全调用 tealdeer，完成最小命令执行通路。  
**任务**：
- 实现 `backend::tealdeer`：system/sidecar 发现、版本读取、`--show-paths` 解析。
- 统一命令执行：`std::process::Command` + 超时 + stdout/stderr 捕获。
- 强制默认 `--raw`，禁用 `--pager`（UI 侧不传或后端忽略）。
- 建立 **Command 安全校验**：包含 `/`、`\` 或 `..` 直接拒绝（用于写文件与执行）。
**交付物**：
- Tauri commands：`detect_backend`、`get_show_paths`、`render_tldr`（仅 raw）。
- 单元测试：命令校验与执行参数构造。
**验收**：调用 `render_tldr(["tar"])` 能返回 Markdown 输出。

---

## 阶段 2：Search UI + Markdown 渲染（3–5 天）
**目标**：Search 面板可用，显示 `tldr --raw` 的 Markdown。  
**任务**：
- 前端实现 Search Panel：输入框、语言/平台选项、运行按钮、错误提示。
- Markdown 渲染组件（轻量 renderer），支持 code/list/blockquote。
- Copy/Preview Raw/Update Cache 按钮占位，Preview Raw 直接展示 raw 文本。
**交付物**：
- Search 页面全链路：UI → Rust → `tldr --raw` → 渲染。
**验收**：输入 `git log` 能渲染对应 Markdown，错误场景提示清晰。

---

## 阶段 3：自定义页创建与预览（4–6 天）
**目标**：支持创建 `.page.md` / `.patch.md`，并预览最终效果。  
**任务**：
- 实现 slug 规则（空白→`-`）+ 安全校验（拒绝 `/`、`\`、`..`）。
- 生成 Markdown 模板（page/patch/append），写入 custom pages dir。
- Preview Effective Output：调用 `tldr --raw <command>`。
**交付物**：
- Tauri commands：`create_or_overwrite_page`、`create_or_overwrite_patch`、`append_example_to_page`、`preview_effective_output`。
- UI New Panel 表单 + 预览按钮。
**验收**：创建后终端 `tldr <cmd>` 立即生效（System 模式）。

---

## 阶段 4：Manage 页面与禁用/启用（4–6 天）
**目标**：管理自定义页，支持禁用/启用/删除/打开。  
**任务**：
- 扫描 custom pages dir：识别 page/patch/disabled，解析 summary 与示例数。
- 实现禁用/启用重命名策略（`.disabled` 后缀），处理重名冲突。
- UI 列表筛选 + 搜索 + 操作（Open/Edit/Delete）。
**交付物**：
- Tauri commands：`scan_custom_pages`、`delete_custom_file`、`disable_custom_file`、`enable_custom_file`、`read_custom_file`。
**验收**：启用/禁用切换后 `tldr` 输出随之变化。

---

## 阶段 5：Settings 与配置隔离（3–5 天）
**目标**：可控写入 tealdeer 配置，Sidecar 完全隔离。  
**任务**：
- System 模式默认只读；Sidecar 模式强制使用 app_data config（`--config-path` 或 `TEALDEER_CONFIG_DIR`）。
- UI Settings 表单仅覆盖承诺字段（language/platform/auto_update/pager/color/archive_source）。
- 提供 “Open config.toml / Show paths” 按钮。
**交付物**：
- `get_tealdeer_config` / `set_tealdeer_config` + Settings UI。
**验收**：切换配置后执行 `render_tldr` 使用隔离配置生效。

---

## 阶段 6：托盘与快捷键（3–5 天）
**目标**：实现常驻小窗体验。  
**任务**：
- 托盘菜单：Show/Hide、Search、New、Update、Open Dir、Settings、Quit。
- 全局快捷键：默认 `Ctrl+Alt+T`，支持在 Settings 修改。
- Wayland/KDE 降级策略：置顶失效时提示并允许关闭置顶。
**交付物**：
- `backend::tray_hotkey` + 前端状态同步。
**验收**：托盘与快捷键可稳定呼出/隐藏窗口。

---

## 阶段 7：打包、测试与发布准备（3–5 天）
**目标**：可在 Linux 上稳定打包与回归测试。  
**任务**：
- Sidecar 打包：配置 `externalBin`，针对目标三元组产物命名。
- 测试计划落实：slug/Markdown/扫描/重命名冲突单测；集成测试调用 `tldr --raw`.
- 文档补全：用户使用说明、常见问题（Wayland/托盘）。
**交付物**：
- 可分发安装包（deb/appimage 视需求）。
- 最小测试集通过，发布说明完善。
**验收**：干净环境可安装运行，Search/New/Manage/Settings/Tray 全部可用。
