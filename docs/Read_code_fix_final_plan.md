# 沉浸式阅读（便利贴模式）实施计划（Final）

> 依据 `Read_code_fix_final.md` 的终极技术方案拆解，按阶段执行，确保最小风险与可验证交付。

---

## 阶段 0：准备与对齐（0.5 天）
### 目标
对齐窗口形态、默认行为与最小可交付范围，避免后续返工。

### 任务
1. 确认沉浸式窗口默认行为  
   - 打开沉浸窗口后主窗口默认动作（隐藏到托盘/最小化/保持原状）。
2. 确认 UI 样式基线  
   - 分组布局、卡片交互、三色圆点过滤、搜索即过滤、左侧贴边目录。
3. 确认数据范围  
   - 沉浸窗口只读取 favorites，不新增收藏字段。

### 验收指标
- 形成明确的需求列表与默认设置项（写入 README 或内部备注）。
- 确认不会新增第三方依赖。

---

## 阶段 1：后端窗口与入口（1 ~ 1.5 天）
### 目标
新增沉浸窗口创建/显示/关闭能力，并提供前端调用入口。

### 任务
1. 新增 IPC 命令  
   - `open_immersive_window`、`close_immersive_window`（如需）。  
2. 新增窗口管理模块（可选独立文件）  
   - `frontend/tealdeer-widget/src-tauri/src/backend/immersive_window.rs`  
   - 管理窗口句柄、打开/聚焦/关闭逻辑。
3. 主窗口行为控制  
   - 默认打开沉浸窗口后隐藏到托盘。
4. 窗口状态保存与恢复  
   - 保存位置/大小/置顶/透明度（默认值与策略一致）。
5. 窗口默认位置策略  
   - 右下角贴边（预留 20px）。

### 新增/修改内容
- 修改：`frontend/tealdeer-widget/src-tauri/src/lib.rs`  
- 修改：`frontend/tealdeer-widget/src-tauri/src/backend/mod.rs`  
- 新增：`frontend/tealdeer-widget/src-tauri/src/backend/immersive_window.rs`（可选）

### 验收指标
- 前端可调用打开沉浸窗口 IPC。
- 窗口可重复打开且不会产生多个实例。
- 主窗口在打开沉浸窗口后按默认策略隐藏到托盘。
- 窗口位置与大小可保存并在下次打开恢复。

---

## 阶段 2：沉浸式页面骨架（1.5 ~ 2 天）
### 目标
完成沉浸式页面 UI 结构与基本渲染。

### 任务
1. 新增沉浸式主组件  
   - `ImmersiveView.vue`：搜索栏 + 三色圆点 + 内容区 + 左侧目录容器。
2. 新增卡片组件  
   - `ImmersiveCard.vue`：显示描述 + 命令/快捷键。
3. 新增目录组件  
   - `ImmersiveSidebar.vue`：目录分组与锚点跳转。

### 新增/修改内容
- 新增：`frontend/tealdeer-widget/src/components/ImmersiveView.vue`  
- 新增：`frontend/tealdeer-widget/src/components/ImmersiveCard.vue`  
- 新增：`frontend/tealdeer-widget/src/components/ImmersiveSidebar.vue`

### 验收指标
- 页面结构完整，分组标题与双列卡片布局生效。
- 卡片内容不折叠、分组可折叠。
- 搜索栏与三色圆点位置符合设计。

---

## 阶段 3：数据接入与过滤逻辑（1 ~ 1.5 天）
### 目标
将 favorites 数据映射到沉浸式视图，完成 All/Command/Shortcut 过滤与实时搜索。

### 任务
1. favorites 数据加载  
   - 调用 `get_favorites`，解析 `scope::title`。
2. 实时过滤  
   - 搜索框输入即过滤：命令/描述/页面名。
3. 模式过滤  
   - 三色圆点切换 All / Command / Shortcut。
4. 收藏变更同步  
   - 监听 `favorites-updated` 并刷新视图。

### 新增/修改内容
- 修改：`ImmersiveView.vue`  

### 验收指标
- 输入即过滤生效，过滤不需要点击按钮。  
- 三色圆点切换能正确更新列表。  
- 默认 All 显示，符合“渐进式披露”。
- 收藏变化后沉浸窗口能即时更新。

---

## 阶段 4：左侧目录与贴边呼出（1 ~ 1.5 天）
### 目标
实现贴边呼出目录 + 快速定位能力。

### 任务
1. 目录生成  
   - Commands 与 Shortcut Keys 分组下列出条目。  
2. 贴边呼出逻辑  
   - 鼠标贴左边缘自动展开，离开区域自动隐藏。
3. 快速定位  
   - 点击目录项滚动到对应卡片（锚点）。

### 新增/修改内容
- 修改：`ImmersiveSidebar.vue`  
- 修改：`ImmersiveView.vue`（挂载目录与滚动锚点）

### 验收指标
- 鼠标贴边 8~12px 内目录自动出现。  
- 点击目录项可定位对应卡片。  
- 目录随过滤模式动态更新。

---

## 阶段 5：交互 / 视觉 / 性能打磨（1.5 ~ 2 天）
### 目标
完成沉浸感设计（卡片动效、轻量配色、复制提示）并保证性能稳定。

### 任务
1. 卡片交互  
   - hover 轻微放大 + 阴影  
   - 点击复制 + toast 提示
2. 颜色编码  
   - 左边框区分 Command / Shortcut。
3. 配色方案落地  
   - 使用低饱和主题色，降低视觉干扰。
4. 搜索防抖  
   - 150~250ms 防抖，避免频繁重排。
5. 分段渲染策略  
   - 收藏量 > 120 时启用分段加载（按分组/分页）。

### 新增/修改内容
- 修改：`ImmersiveCard.vue`  
- 修改：`ImmersiveView.vue` 样式  

### 验收指标
- 卡片 hover 动效明显但不过度。  
- 点击卡片复制成功有提示。  
- Command/Shortcut 通过颜色可清晰区分。
- 搜索输入流畅不卡顿。  
- 收藏量 100+ 仍保持滚动流畅。

---

## 阶段 6：配置项与行为一致性（0.5 ~ 1 天）
### 目标
让沉浸模式行为可配置，且与主窗口逻辑一致。

### 任务
1. 新增设置项  
   - `immersive_on_open_action`（hide_to_tray / minimize / none）  
   - `immersive_always_on_top`  
   - `immersive_opacity`  
   - `immersive_default_mode`（all / command / shortcut）  
   - `immersive_sidebar_default`（true / false）
2. 默认策略落地  
   - 默认值符合设计：隐藏到托盘、置顶开启、透明度 1.0。

### 新增/修改内容
- 修改：`frontend/tealdeer-widget/src-tauri/src/backend/settings.rs`  
- 修改：`frontend/tealdeer-widget/src/App.vue`（入口控制）

### 验收指标
- 配置项可写入并读取。  
- 不改动现有配置结构的稳定性。  
- 沉浸窗口默认行为与配置一致。

---

## 阶段 7：引导与错误处理（0.5 ~ 1 天）
### 目标
保证沉浸模式在异常场景下有稳定反馈，且不破坏沉浸体验。

### 任务
1. 空状态  
   - favorites 为空时显示简洁提示。
2. 加载失败  
   - Toast 提示 + 回退空数据。
3. 复制失败  
   - Toast 提示 + 日志输出。
4. 窗口创建失败（可选）  
   - 回退到主窗口“沉浸区域”或提示失败。

### 验收指标
- 空状态显示符合视觉风格。  
- 异常失败有明确提示但不干扰阅读。  
- 错误不会导致窗口崩溃。

---

## 阶段 8：验收与回归测试（0.5 ~ 1 天）
### 目标
保证沉浸模式稳定，不破坏主流程。

### 任务
1. 功能回归  
   - 主窗口功能无退化  
   - favorites 修改后沉浸窗口更新  
2. 交互回归  
   - 搜索即过滤  
   - 目录贴边呼出  
   - 卡片点击复制  
3. 性能验证  
   - 收藏量 100+ 时滚动流畅

### 验收指标
- 通过关键路径测试（手工验证）。  
- 运行 `cargo check` 无编译错误（如涉及 Rust 变更）。  

---

## 交付标准（最终验收）
- 沉浸窗口独立且稳定  
- 搜索即过滤 + 三色圆点切换  
- Commands/Shortcuts 双列卡片完整显示  
- 卡片点击复制与提示  
- 左侧目录贴边呼出 + 快速定位  
- 主窗口默认隐藏到托盘，可配置  
- 不引入新依赖，保持项目一致性
- 空状态与错误处理完整
