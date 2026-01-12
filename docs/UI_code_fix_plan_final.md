# UI 最终改造方案（收藏 + 代码块增强）

本方案为最终落地版本，满足“不修改 TLDR 原始 Markdown”的硬性约束，仅对渲染视图做增强，Raw 视图保持原样。

## 0. 方案定位
- 采用“结构化渲染 + 后端持久化收藏”方案。
- 渲染区从 `v-html` 迁移为 Vue 模板渲染，确保交互可控、易维护。
- 收藏数据存储在 Tauri app data 目录的 `favorites.json`，保证跨平台一致与可移植。

## 1. 详细技术方案设计

### 1.1 收藏功能
- **存储位置**：Tauri `app_data_dir()`，如 `~/.local/share/com.xian00.tealdeer-tile/favorites.json`
- **分类策略**：按 TLDR 页面标题分组（如 `pacman`、`git log`）
- **去重策略**：命令字符串标准化后比对（trim + 空白折叠）
- **交互行为**：
  - 已收藏命令按钮置灰（disabled）
  - tooltip 文案：已收藏
  - 批量收藏只添加未收藏项

### 1.2 渲染增强（不改 Markdown）
解析 raw Markdown 为结构化数据，再用 Vue 组件渲染：
- 标题：`# ...` 或 v2 `title + =====`
- 描述：`> ...`
- 示例：`- 描述` + 下一行 `` `命令` ``

渲染增强内容：
- 行号
- 每行 `[copy][favorite]`
- 页头 `[copy all][favorite all]`
- 语法高亮（自定义 tokenizer）

### 1.3 语法高亮
通过轻量 tokenizer 识别以下 token 并加 class：
- 命令名（首 token）
- `sudo`
- 参数选项（`-X` / `--flag`）
- 变量（`{{...}}`）
- 字符串（`"..."` / `'...'`）
- 操作符（`|` `&&` `;` `>` `<`）
- 路径（`/path/...`）

### 1.4 复制/收藏反馈
使用现有 toast 机制：
- `copy` 成功提示
- `favorite` 成功提示
- 批量收藏成功提示

## 2. 数据结构设计

### 2.1 收藏 JSON
```json
{
  "version": 1,
  "updated_at": 1730000000,
  "items": {
    "pacman": [
      "sudo pacman -Syu",
      "sudo pacman -S {{软件包}}"
    ]
  }
}
```

### 2.2 前端类型（示意）
```ts
type ParsedPage = {
  title: string;
  description: string;
  examples: { desc: string; cmd: string }[];
};

type Favorites = {
  version: number;
  updated_at: number;
  items: Record<string, string[]>;
};
```

### 2.3 去重索引
```ts
type FavoriteIndex = Record<string, Set<string>>; // page -> normalized command
```

## 3. 交互原型（文本版）

### 3.1 渲染页面
```
pacman  [copy all] [favorite all]
Arch Linux 的软件包管理器工具...

1  sudo pacman -Syu                    [copy] [favorite]
2  sudo pacman -S {{软件包}}           [copy] [favorite]
3  pacman -Qe                          [copy] [favorite]
```

### 3.2 已收藏状态
```
1  sudo pacman -Syu                    [copy] [favorited]
```
说明：`favorited` 置灰、不可点击、tooltip “已收藏”

### 3.3 Settings - 我的收藏
```
我的收藏（可折叠）
  pacman (3)
    sudo pacman -Syu     [copy] [remove]
    pacman -Qe           [copy] [remove]
  git log (2)
    git log --stat       [copy] [remove]
```

## 4. 模块拆分（更细）

### 4.1 前端（Vue）
- `frontend/tealdeer-widget/src/utils/tldr_parser.ts`
  - raw Markdown → ParsedPage
- `frontend/tealdeer-widget/src/utils/command_tokens.ts`
  - tokenizer & 高亮 class
- `frontend/tealdeer-widget/src/stores/favorites.ts`
  - 收藏加载、写入、去重索引
- `frontend/tealdeer-widget/src/components/RenderedPage.vue`
  - 结构化渲染区（替换 v-html）
- `frontend/tealdeer-widget/src/components/CommandLine.vue`
  - 行号 + 命令 + 按钮
- `frontend/tealdeer-widget/src/components/FavoritesPanel.vue`
  - Settings 收藏列表
- `frontend/tealdeer-widget/src/locales/en.json`
- `frontend/tealdeer-widget/src/locales/zh.json`

### 4.2 后端（Tauri Rust）
- `frontend/tealdeer-widget/src-tauri/src/backend/favorites.rs`（新增）
  - 读写 favorites.json
  - 去重逻辑
- `frontend/tealdeer-widget/src-tauri/src/backend/mod.rs`
  - 注册 favorites 模块
- `frontend/tealdeer-widget/src-tauri/src/lib.rs`
  - `invoke_handler` 注册 favorites 命令

## 5. 分阶段改造计划（分步骤）

### 阶段 1：收藏后端与基础 API
1. 新增 `favorites.json` 读写模块（Rust）。
2. 增加 Tauri 命令：get/add/remove/clear。
3. 处理原子写入与去重。

### 阶段 2：前端收藏状态
1. 新增 `favorites` store。
2. App 启动加载收藏列表。
3. 完成收藏去重索引与禁用逻辑。

### 阶段 3：结构化渲染迁移
1. 新增 `tldr_parser.ts` 解析 raw。
2. Rendered 区切换为 `RenderedPage.vue` 模板渲染。
3. 添加行号与按钮布局。

### 阶段 4：语法高亮与交互完善
1. 实现 tokenizer 与 CSS 高亮。
2. 加入 copy/favorite 的 toast。
3. 批量 copy/favorite 行为。

### 阶段 5：Settings 收藏列表
1. 新增 FavoritesPanel。
2. 支持分组、复制、移除。
3. 空态与样式细节。

### 阶段 6：回归与优化
1. 多语言与布局测试。
2. 复制/收藏边界情况验证。
3. 性能与交互细节调优。



