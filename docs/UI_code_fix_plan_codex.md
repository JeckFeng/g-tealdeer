# UI 优化方案（收藏 + 代码块增强）

本文档说明“命令收藏功能”和“渲染代码块增强”的技术方案。所有改造都**不修改 TLDR 原始 Markdown**，只对渲染结果做增强；Raw 视图保持原样。

## 目标
- 增加命令收藏功能，并按页面标题自动分类。
- 渲染代码块增加行号、复制/收藏按钮、全页复制/收藏按钮。
- 语法高亮更丰富，提升可读性。
- 已收藏的命令不可重复收藏（按钮置灰、不可点击）。

## 约束
- 禁止修改 TLDR Markdown 源文件。
- 增强仅作用于“渲染视图”，Raw 保持不变。
- 交互与样式均在前端渲染层完成。

## 收藏数据存储
### 存储位置
使用 Tauri 的 `app_data_dir()`，例如：
`~/.local/share/com.xian00.tealdeer-tile/favorites.json`

### 数据格式（建议）
```json
{
  "version": 1,
  "updated_at": 1730000000,
  "items": {
    "pacman": [
      "sudo pacman -Syu",
      "sudo pacman -S {{软件包}}",
      "pacman -Qe"
    ],
    "git log": [
      "git log --stat",
      "git log --oneline --graph"
    ]
  }
}
```

### 自动分类
- 分类 key 使用 TLDR 页面标题（如 `# pacman` → `pacman`）。
- 如果解析失败，回退到当前搜索输入（`lastCommand`）。

### 去重策略（必须）
为避免重复收藏，需要**标准化命令字符串**用于匹配：
- `trim()` 去首尾空格
- 内部连续空格折叠为单空格
- 仅用于匹配，显示仍保留原样

当命令已存在于收藏：
- 收藏按钮置灰（disabled）
- 设置 `aria-disabled="true"`
- 提示文案：已收藏

## 后端接口（Tauri Rust）
新增收藏 API（示例）：
- `get_favorites() -> Favorites`
- `add_favorite(page: String, command: String)`
- `remove_favorite(page: String, command: String)`
- `clear_favorites(page: Option<String>)`

实现要求：
- 自动创建目录
- 读写 JSON
- 原子写入（写临时文件再 rename）
- 写入时做去重处理

## 前端状态设计
建议状态结构：
- `favoritesByPage: Record<string, string[]>`
- `favoriteIndex: Record<string, Set<string>>`（标准化后的快速匹配）
- `activePageTitle: string`

加载流程：
1. App 初始化时读取 favorites。
2. 渲染页面时生成当前页面的命令列表并建立索引。
3. 点击收藏时先查 `favoriteIndex` 去重。
4. 更新成功后刷新本地状态并提示成功。

## 代码块增强（不改 Markdown）
### 核心思路
用解析器将 raw Markdown 转为结构化数据，再由 Vue 模板渲染：
- 标题 `# ...`
- 描述 `> ...`
- 示例 `- 描述` + 下一行 `` `命令` ``

结构示例：
```ts
{
  title: "pacman",
  description: "Arch Linux 的软件包管理器工具。",
  examples: [
    { desc: "同步并更新所有软件包：", cmd: "sudo pacman -Syu" },
    { desc: "安装一个新的软件包：", cmd: "sudo pacman -S {{软件包}}" }
  ]
}
```

### 渲染增强
每条命令渲染为结构化行：
- 行号
- 命令文本（带高亮）
- `[copy]` 按钮
- `[favorite]` 按钮（已收藏则置灰）

页头增加：
- `[copy all]` 全部复制
- `[favorite all]` 批量收藏（已收藏的不重复加入）

### 语法高亮增强
对命令做轻量 tokenizer，并为不同 token 设置 class：
- 命令名（首 token）
- `sudo`
- 参数选项：`-X` / `--flag`
- 变量：`{{...}}`
- 字符串：`"..."` / `'...'`
- 操作符：`|` `&&` `;` `>` `<`
- 路径：`/path/...`

通过 CSS 提供更丰富的颜色。

## Settings 页面：收藏列表布局
在 Settings 中新增一个独立可折叠容器：
```
Settings
  我的收藏（可折叠）
    pacman (3)
      sudo pacman -Syu     [copy] [remove]
      pacman -Qe           [copy] [remove]
    git log (2)
      git log --stat       [copy] [remove]
```

建议功能：
- 分组显示（按页面标题）
- 每条命令提供复制/移除
- 分组标题显示收藏数量

## 交互与提示
- 复制成功 → toast 提示
- 收藏成功 → toast 提示
- 已收藏 → 收藏按钮置灰 + 提示“已收藏”
- 批量收藏 → 只添加未收藏项

## 交互原型（文本版）
### 搜索页/新建页渲染区
```
pacman  [copy all] [favorite all]
Arch Linux 的软件包管理器工具...

1  sudo pacman -Syu                    [copy] [favorite]
2  sudo pacman -S {{软件包}}           [copy] [favorite]
3  pacman -Qe                          [copy] [favorite]
```

### 收藏按钮状态
- 未收藏：主色按钮，hover 显示提示“收藏”
- 已收藏：灰色禁用态，hover 提示“已收藏”，不可点击

### Settings - 我的收藏
```
我的收藏（可折叠）
  pacman (3)
    sudo pacman -Syu     [copy] [remove]
    pacman -Qe           [copy] [remove]
  git log (2)
    git log --stat       [copy] [remove]
```

## 模块拆分（更细）
### 前端（Vue）
- `src/utils/tldr_parser.ts`
  - 输入：raw Markdown
  - 输出：`{ title, description, examples[] }`
  - 兼容 v1/v2 标题格式
- `src/utils/command_tokens.ts`
  - 轻量 tokenizer，为高亮提供 token + class
- `src/stores/favorites.ts`
  - 读取/写入收藏
  - 维护 `favoritesByPage` 与 `favoriteIndex`
- `src/components/RenderedPage.vue`（可选拆分）
  - 接收结构化数据渲染页面
  - 命令行渲染、行号、按钮
- `src/components/FavoritesPanel.vue`（可选拆分）
  - Settings 里的收藏列表
- `src/locales/en.json` / `src/locales/zh.json`
  - 新增提示文案：收藏成功、已收藏、复制成功等
- `App.vue`
  - 保留整体布局与切换逻辑
  - 调用解析、渲染组件与收藏状态

### 后端（Tauri Rust）
- `src-tauri/src/backend/favorites.rs`（新增）
  - 读写 favorites.json
  - 去重逻辑
- `src-tauri/src/backend/mod.rs`
  - 注册 favorites 模块
- `src-tauri/src/lib.rs`
  - `invoke_handler` 注册 favorites 命令

### 数据结构（示意）
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

## 分阶段改造计划（分步骤）
### 阶段 1：收藏存储与基础 API
1. 增加 `favorites.json` 读写与去重逻辑（Rust 后端）。
2. 添加 Tauri 命令：get/add/remove/clear。
3. 前端加载收藏列表并缓存到内存。

### 阶段 2：解析与结构化渲染
1. 前端实现 `tldr_parser.ts` 解析 raw Markdown。
2. Rendered 视图改为结构化渲染（不再使用 `v-html`）。
3. 添加行号与基本按钮布局（copy/favorite）。

### 阶段 3：收藏交互与去重
1. 收藏按钮与状态绑定 `favoriteIndex`。
2. 已收藏禁用 + tooltip。
3. 批量收藏：只添加未收藏项。

### 阶段 4：语法高亮与 UI 完成度
1. 实现 tokenizer 并加颜色样式。
2. Settings 增加“我的收藏”列表。
3. 增加 toast 文案与 i18n。

### 阶段 5：回归与体验调优
1. 多语言/平台下的布局与空态验证。
2. 复制/收藏/删除全流程验证。
3. 性能与可用性微调。

## 边界情况
- 不同页面同名命令可分别收藏
- Raw 输出为空时隐藏增强按钮
- 收藏列表为空时显示空态

## 改造范围总结
- 新增 favorites 存储与 API（Tauri Rust）
- 前端状态管理与收藏逻辑
- 渲染视图改为结构化模板
- Settings 添加收藏列表容器
- 收藏按钮状态去重处理
