# 沉浸式阅读（便利贴模式）终极技术方案

## 1. 目标与范围
- 新增 **独立沉浸窗口**（便利贴模式）
- 仅展示收藏内容（Commands / Shortcut Keys），不引入管理/编辑
- 默认 All 展示，必要时再过滤
- 主窗口打开后 **默认隐藏到托盘**（可配置）

**非目标**
- 不新增收藏数据格式字段
- 不引入新依赖（VueUse 等）
- 不改变原有主窗口架构

---

## 2. 架构与窗口管理（细化）
### 2.1 独立窗口
- Tauri 新建 `label = "immersive"` 窗口
- 默认位置：右下角贴边（留 20px 边距）
- 默认置顶：开启
- 透明度：可配置（1.0 ~ 0.7）
- 可拖拽、可缩放

### 2.2 窗口生命周期
- 打开流程：
  1) 若窗口已存在 → 聚焦并置顶  
  2) 若不存在 → 创建窗口 → 加载内容 → 显示  
- 关闭流程：
  1) 保存位置与大小  
  2) 关闭窗口  
  3) 主窗口按策略恢复或保持隐藏

### 2.3 主窗口行为（默认策略）
打开沉浸窗口后：
- **隐藏到托盘**（默认）
- 可配置：最小化 / 保持原状

---

## 3. UI 布局与交互
### 3.1 布局结构
```
[三色圆点过滤]   [搜索框：输入即过滤]
-------------------------------------------------
 Commands (双列卡片)
  ├─ card
  ├─ card
 Shortcut Keys (双列卡片)
  ├─ card
  ├─ card
-------------------------------------------------
 左侧贴边目录栏（鼠标贴边呼出）
```
规则：
- 搜索栏无按钮，输入即过滤
- 分组标题可折叠（仅 Commands / Shortcut Keys）
- 卡片内容不折叠，保证“一眼可见”

### 3.2 交互逻辑
**搜索与过滤**
- 默认 All  
- 三色圆点切换：All / Commands / Shortcut Keys  
- 搜索框实时过滤（命令、描述、页面名）

**卡片交互**
- hover 轻微放大 + 阴影  
- 点击卡片：复制内容 + toast 提示  
- 不触发展开/折叠（避免误操作）

**目录栏**
- 左边缘 8~12px 细线/小标签  
- 鼠标贴边呼出，离开自动隐藏  
- 目录随过滤模式动态更新  
- 点击目录项滚动定位

---

## 4. 数据结构与派生逻辑
### 4.1 收藏数据（复用现有）
```ts
type Favorites = {
  version: number;
  updated_at: number;
  items: Record<string, { command: string; description: string }[]>;
};
```

### 4.2 派生结构（沉浸视图）
```ts
type ImmersiveItem = {
  scope: "command" | "shortcut";
  pageTitle: string;
  command: string;
  description: string;
  anchorId: string;
};
```

派生规则：
- `scope::title` → scope + pageTitle  
- `anchorId = scope + "-" + pageTitle + "-" + index`  
- 搜索过滤：命令/描述/pageTitle 维度

---

## 5. 事件与同步
后端在收藏变更时广播：
- `favorites-updated`  
沉浸窗口监听刷新，主窗口保持现有逻辑。

---

## 6. 性能优化（细化）
1) **批量渲染策略**  
   - 收藏量 > 120 时启用分段渲染（按分组分页加载）  
2) **搜索输入防抖**  
   - 150~250ms 防抖，避免频繁重排  
3) **目录项虚拟化（可选）**  
   - 目录条目过长时仅渲染可见区域  
4) **图片与图标**  
   - 避免大图，使用内联 SVG 或 icon font（已有资源优先）

---

## 7. 配色方案与层级
### 7.1 基础色
```css
--immersive-bg: #faf7f2;
--immersive-card: #ffffff;
--immersive-border: #e7e2da;
--command-accent: #4f87ff;
--shortcut-accent: #f7a34b;
--text-primary: #2a2a2a;
--text-muted: #6b6b6b;
```

### 7.2 视觉层次
- 分组标题：字体更大 + 深色  
- 卡片边框：轻量颜色编码  
- 卡片阴影：轻微浮起，避免强烈层次冲突

---

## 8. 引导与可用性（细化）
**轻量引导（可选，首次打开）**
- 显示 3 条小提示：  
  1) 输入即过滤  
  2) 点击卡片复制  
  3) 左侧贴边呼出目录  
**设计原则**：提示不遮挡主内容，3 秒后自动淡出。

---

## 9. 错误处理策略
1) **收藏为空**  
   - 显示“暂无收藏”的空状态  
2) **加载失败**  
   - Toast 提示 + 回退空数据  
3) **复制失败**  
   - Toast 提示并输出日志  
4) **窗口创建失败**  
   - 回退到主窗口内“沉浸模式区域”（可选）

---

## 10. 配置项（细化）
新增设置项（写入 AppSettings）：
- `immersive_on_open_action`: `hide_to_tray | minimize | none`
- `immersive_always_on_top`: `true | false`
- `immersive_opacity`: `1.0 ~ 0.7`
- `immersive_default_mode`: `all | command | shortcut`
- `immersive_sidebar_default`: `true | false`

默认值：
- `hide_to_tray`  
- `always_on_top = true`  
- `opacity = 1.0`  
- `default_mode = all`  
- `sidebar_default = true`

---

## 11. 文件变更清单
**修改**
- `frontend/tealdeer-widget/src/App.vue`  
- `frontend/tealdeer-widget/src-tauri/src/lib.rs`  
- `frontend/tealdeer-widget/src-tauri/src/backend/settings.rs`  
- `frontend/tealdeer-widget/src-tauri/src/backend/tray_hotkey.rs`  
- `frontend/tealdeer-widget/src/locales/en.json`  
- `frontend/tealdeer-widget/src/locales/zh.json`

**新增**
- `frontend/tealdeer-widget/src/components/ImmersiveView.vue`
- `frontend/tealdeer-widget/src/components/ImmersiveCard.vue`
- `frontend/tealdeer-widget/src/components/ImmersiveSidebar.vue`
- `frontend/tealdeer-widget/src-tauri/src/backend/immersive_window.rs`（可选）

---

## 12. 风险与对策
- **多窗口通信**：统一事件名 + 单向广播  
- **性能问题**：大列表启用分段渲染  
- **沉浸感破坏**：控件最少化，提示轻量化  

---

## 13. 最终结论
该方案完全对齐你给出的设计原则与交互要求，保持现有项目结构不变，新增独立沉浸窗口并强化窗口管理、性能、配色、引导与错误处理细节，确保可持续迭代与低风险落地。 
