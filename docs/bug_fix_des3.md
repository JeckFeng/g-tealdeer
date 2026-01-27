# Bug Fix Design 3：子命令回退 + 示例过滤（继承原版 tldr 搜索机制）

## 背景与目标

当前 tealdeer‑tile 在 CLI / GUI 渲染时，会将输入的多 token 命令直接用 `-` 拼接成页面名进行查找（如 `systemctl stop` → `systemctl-stop`）。当页面不存在时会直接报错；而原版 tldr 客户端在相同输入下，会**回退到主页面**（`systemctl`）并**按子命令关键字过滤示例**，从而给出“Stop …”相关的片段。

**目标**：
1. **完全继承原版 tldr 的多 token 搜索行为**：如果 `systemctl-stop` 不存在，则回退到 `systemctl` 并过滤示例。
2. **对所有命令生效**（不仅 `systemctl`）。
3. **保留并扩展** tealdeer‑tile 的快捷键搜索、自定义、收藏功能。
4. **不破坏现有渲染与缓存机制**，并保证 UI/CLI 行为一致。

---

## 实现难度评估

**综合难度：中等偏高**

原因：
- 需要在 **核心渲染路径**引入“回退 + 示例过滤”机制，影响 CLI 与 GUI 行为。
- 需要在 **渲染层**基于 TLDR Markdown 解析结果做筛选（不能直接过滤纯文本输出）。
- 需要兼容 **自定义页面 / patch / shortcut pages** 目录结构。
- 需要明确 **过滤策略** 与 **回退规则**，避免引入新的误匹配。

预计工作量：
- 后端（Rust core + tauri backend）2–3 天
- 前端（Vue UI / i18n / 交互逻辑）1–2 天
- 测试与回归 1 天

---

## 技术方案概览

### 核心思路

1. **解析命令 tokens**：
   - 输入 `systemctl stop foo` → tokens = `[systemctl, stop, foo]`
2. **优先查找完整页面**：
   - 先尝试 `systemctl-stop-foo` → `systemctl-stop` → `systemctl`（逐步回退）
3. **回退后过滤示例**：
   - 如果回退到更短页面（如 `systemctl`），则对示例进行过滤：仅保留包含剩余 tokens 的示例
4. **返回过滤后的渲染结果**：
   - CLI 输出 + GUI 渲染均基于过滤后的示例

---

## 详细设计

### 1) 回退规则（Fallback Resolution）

输入 tokens：`[t0, t1, ..., tn]`

按以下顺序查找页面：
1. `t0-t1-...-tn`
2. `t0-t1-...(n-1)`
3. ...
4. `t0`

**找到第一个存在的页面后停止。**

- 如果命中的是完整页面（步骤 1），则不需要过滤。
- 如果命中的是回退页面（步骤 2+），则启用过滤：
  - 过滤关键字 = 被“截断”的 tokens（例如：`systemctl stop` 命中 `systemctl`，过滤词 = `stop`）

### 2) 示例过滤规则（Example Filtering）

输入：TLDR page 解析后的 `examples` 列表。

过滤策略：
- **默认使用 AND 匹配**：示例文本（描述 + 命令）必须包含所有过滤词。
- 支持 `--or` 策略作为可选参数（未来扩展）。
- 过滤时做以下归一化：
  - 全部小写
  - 空格与连字符统一处理（如 `git-checkout` == `git checkout`）
  - 允许部分匹配（`stop` 可以命中 `--stop`）

### 2.1) 过滤实现细节（更可执行）

**解析层选择**：在 `tealdeer-core` 侧基于 `LineIterator + LineType` 做过滤，避免重复实现 Markdown 解析器。  
思路是“只过滤 ExampleText/ExampleCode 对，其它行原样保留”。  

**匹配粒度**：  
- 将过滤词和示例文本统一为小写  
- `空格` 和 `-` 统一处理  
- 示例文本构成 = `ExampleText + ExampleCode` 拼接字符串（既匹配描述，也匹配命令）  

**示例过滤伪代码**：
```
filters = ["stop", "user"]  // 由回退 tokens 得到
for line in LineIterator(page):
  if line is ExampleText:
     current_example_text = line.text
  if line is ExampleCode:
     current_example_code = line.code
     example_blob = normalize(current_example_text + " " + current_example_code)
     if all filters in example_blob:
         emit ExampleText + ExampleCode
     else:
         skip
  else:
     emit line
```

**空结果策略**：
- 如果过滤后示例为空：  
  - CLI：输出提示 `No examples matched filters: ...`  
  - GUI：显示提示，并提供“显示完整页面”按钮  

### 2.2) 回退后过滤触发条件

- **仅当命中页面不是最完整页面** 才启用过滤  
  - 完整命中（`t0-t1-...-tn`） → 不过滤  
  - 回退命中（`t0-...-tk`） → 过滤词 = `t(k+1)...tn`

### 2.3) 策略扩展（可选）

- `--fallback-filter=and|or` CLI 参数  
- 配置文件 `search.subcommand_filter_mode = "and"`（默认）  
- GUI 提供 `AND/OR` 切换（后期）

### 3) CLI 行为一致性

CLI 输出分两种：
1. **完整命中**：直接输出页面
2. **回退过滤**：输出过滤后的页面，并在 stderr/notice 提示：
   - `No page for systemctl-stop, showing systemctl (filtered by: stop)`

### 4) GUI 行为一致性

GUI 调用 render 时同样走回退流程：
- 输出被过滤后的 Markdown
- UI 顶部提示：`Showing systemctl (filtered by: stop)`

---

## 代码改造计划（模块拆分）

### A. tealdeer-core（核心逻辑）

**新增数据结构：**
```rust
struct ResolvedPage {
  lookup: PageLookupResult,
  resolved_name: String,   // 实际命中页面
  requested_name: String,  // 用户请求（t0-...-tn）
  filters: Vec<String>,    // 需要过滤的 tokens
  used_fallback: bool,
}
```

**新增函数：**
- `resolve_page_with_fallback(tokens: &[String], cache: &Cache) -> Option<ResolvedPage>`  
  - 逐步缩短 tokens（n..1）进行 `cache.find_page()`  
  - 一旦命中立即返回，并记录 `filters`
- `filter_example_lines(lines: impl Iterator<Item=LineType>, filters: &[String]) -> Vec<LineType>`

**修改点：**
- `run()` 里原有的 `cache.find_page(command)` 改为：  
  - `resolved = resolve_page_with_fallback(tokens, cache)`  
  - 若 `resolved.used_fallback` 且 `filters` 非空 → 走过滤渲染分支
- `render_page_to_output()` 增加可选过滤参数（仅回退时传入）
- `render_page_to_string()` 与 `output::render_page()` 支持“过滤版 LineIterator”

**关键伪代码：**
```
tokens = args.command
requested = join("-", tokens)
resolved = resolve_page_with_fallback(tokens, cache)
if !resolved: not found
if resolved.used_fallback && !resolved.filters.is_empty():
   render_filtered(resolved.lookup, resolved.filters)
else:
   render_normal(resolved.lookup)
```

### B. tauri backend

**修改：**
- `render_tldr` 增加 metadata 字段：  
  - `fallback_from`: Option<String>  
  - `filters`: Vec<String>  
- 仅在 fallback 发生时赋值（避免噪音）

### C. 前端（Vue）

**修改：**
- Search/Render 输出顶部增加提示条  
  - `Showing {resolved} (filtered by: stop, user)`  
- 过滤结果为空时：显示“无匹配”提示 + “显示完整页面”按钮  
- i18n：新增提示文案（fallback 提示、空匹配提示）

### D. Tests

1. **单元测试**  
   - `resolve_page_with_fallback`：覆盖多层回退  
   - `filter_example_lines`：AND / 空结果 / 连字符匹配  
2. **集成测试**  
   - CLI：`systemctl stop` 仅显示 stop 示例  
   - GUI：`git checkout` 触发 fallback + 提示  

---

## 兼容性说明

- 不影响已有完整页面的查询逻辑。
- 对 shortcut pages 不启用 fallback 过滤（保持独立逻辑）。
- 对 custom pages / patch 页面同样适用。

---

## 预期用户体验

- `tldr systemctl stop` → 自动输出 systemctl 页面中 stop 相关部分
- `tldr git checkout` → 若不存在 `git-checkout` 则回退 `git` 并过滤包含 checkout 的示例
- GUI 搜索与 CLI 一致

---

## 风险与缓解

| 风险 | 说明 | 缓解策略 |
|------|------|---------|
| 过滤过严 | AND 过滤导致无结果 | 如果过滤后为空，提示“无匹配”，并回显原页面入口 |
| 过滤误匹配 | 关键词命中不相关示例 | 支持更严格匹配模式（整词匹配） |
| 性能问题 | 大页面过滤较慢 | 过滤只对回退页触发，且示例数量有限 |

---

## 分阶段 / 分步骤实施计划

### 阶段 0：方案冻结与准备（0.5 天）
1. 明确 fallback 行为、过滤策略、空结果策略  
2. 确认 i18n 文案与 GUI 提示样式  
3. 标注涉及文件与 API 变更点

**验收指标**：方案冻结、风险点明确。

---

### 阶段 1：核心回退解析（1 天）
1. `ResolvedPage` 结构体落地  
2. 实现 `resolve_page_with_fallback()`  
3. `run()` 中替换单次 `find_page` 为 fallback

**验收指标**：  
- `systemctl stop` 不再直接报错  
- 能命中 `systemctl` 页面（先不过滤）

---

### 阶段 2：示例过滤（1 天）
1. 基于 `LineIterator + LineType` 实现过滤  
2. 在渲染路径加入过滤入口  
3. 空过滤结果提示

**验收指标**：  
- `systemctl stop` 仅输出 stop 相关示例  
- 空过滤时有明确提示

---

### 阶段 3：后端元数据扩展（0.5 天）
1. tauri `render_tldr` 增加 fallback 元数据  
2. CLI 输出 fallback 提示（stderr/notice）

**验收指标**：  
- GUI/CLI 能显示“回退 + 过滤”提示

---

### 阶段 4：前端展示与交互（0.5 天）
1. 输出区顶部 fallback 提示条  
2. 过滤为空时显示“显示完整页面”按钮  
3. i18n 文案补全

**验收指标**：  
- GUI 能清晰展示 fallback 语义  

---

### 阶段 5：测试与回归（1 天）
1. 单元测试：resolve + filter + empty query  
2. 集成测试：systemctl / git / docker  
3. Shortcut 页不受影响验证

**验收指标**：  
- `cargo test` 全通过  
- 关键场景回归通过

---

## 结论

该方案能实现与原版 tldr 一致的“多 token 子命令过滤”行为，同时保持 tealdeer‑tile 的快捷键扩展功能独立，不破坏现有架构。整体改造规模中等，但收益明显、体验一致。建议按此方案推进。
