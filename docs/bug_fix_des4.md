# Bug Fix Design 4：Shortcut 模式子集匹配 + New Page 示例卡片/按钮重构（液体玻璃 UI）

## 目标

本方案覆盖三个明确需求：
1) Shortcut 模式搜索支持 **token 子集匹配**（仍自动渲染，避免“精确匹配”过严导致找不到 `neovim close tab`）。
2) New Page 示例操作按钮重排：移除顶部“添加示例”，每条示例最右侧提供 “+ / ×” 圆形按钮；Append 模式只保留一个示例，不允许插入/删除。
3) 示例之间加入 **玻璃拟态卡片**（Glassmorphism + Liquid Glass），带 hover 悬浮滤镜效果。

---

## 1. Shortcut 模式：Token 子集匹配（自动渲染）

### 1.1 现状与问题
- 现状：Shortcut 模式为了避免摘要误命中，使用“严格 slug 等于 query”的精确匹配策略。
- 问题：`neovim close` 无法命中 `neovim-close-tab`。

### 1.2 规则定义（子集匹配）

**规则**：
- 将 query 拆分为 tokens（空格/连字符分隔）。
- 将候选 slug 拆分为 tokens（空格/连字符分隔）。
- 若 **query tokens 全部存在于 slug tokens 中**（子集），则认为匹配。

**示例**：
- `neovim close` → tokens: [neovim, close] → matches `neovim-close-tab`。
- `git checkout` → tokens: [git, checkout] → matches `git-checkout`。

### 1.3 技术实现

**位置**：`frontend/tealdeer-widget/src/App.vue`

**核心逻辑**（伪代码）：
```
function tokenizeQuery(value) => lower-case, split by whitespace or '-'
function isTokenSubset(queryTokens, slugTokens)

if pageScope == shortcut:
  results = invoke(search_pages, scope=shortcut)
  candidate = results.find(entry => isTokenSubset(queryTokens, slugTokens(entry.name)))
  if candidate:
     renderCommandWithScope(candidate.name, "shortcut")
  else:
     show noResultsShortcut
```

### 1.4 注意点
- 仍保留摘要过滤（只允许 name 命中），避免误渲染。
- 子集匹配优先级：按 search_pages 返回顺序选择第一个匹配项。

---

## 2. New Page：示例按钮重构（插入/删除）

### 2.1 需求目标
- 移除顶部“添加示例”按钮。
- 每条示例最右侧显示两个圆形按钮：
  - 绿色 “+”：在该示例下插入新示例
  - 红色 “×”：删除该示例
- Append 模式：只保留一个示例输入框，不允许插入/删除。

### 2.2 技术实现

**位置**：`frontend/tealdeer-widget/src/App.vue`

**修改点**：
1. 移除 `.examples-header` 中的 “Add Example” 按钮。
2. 为 `examples` 列表新增 `insertExample(index)` 方法：
   - `examples.splice(index + 1, 0, { desc: '', cmd: '' })`
3. `removeExample(index)` 保持现有逻辑，但隐藏按钮时机：
   - `newMode === 'append'` 时完全隐藏
   - `examples.length === 1` 时隐藏删除

**示例模板结构**：
```
<div class="example-row">
  <div class="example-fields"> ...desc/cmd inputs... </div>
  <div class="example-actions">
     <button class="icon add">+</button>
     <button class="icon remove">×</button>
  </div>
</div>
```

---

## 3. 示例卡片：玻璃拟态 + 液体玻璃效果

### 3.1 设计风格
- **Glassmorphism Cards**：半透明、毛玻璃、背景模糊
- **Liquid Glass**：轻微折射感 + hover 动效

### 3.2 CSS 关键参数（建议）

参考用户给定设计参数映射：
- blurAmount: `6–12px`
- saturation: `1.4`
- displacementScale: 轻度
- aberrationIntensity: 低
- elasticity: 轻
- cornerRadius: `16px`

### 3.3 CSS 示例
```
.example-row {
  background: rgba(255,255,255,0.25);
  border: 1px solid rgba(255,255,255,0.35);
  backdrop-filter: blur(10px) saturate(1.4);
  border-radius: 16px;
  padding: 12px 14px;
  box-shadow: 0 8px 20px rgba(0,0,0,0.08);
  transition: transform 0.2s ease, filter 0.2s ease, box-shadow 0.2s ease;
}

.example-row:hover {
  transform: translateY(-2px);
  filter: saturate(1.3) contrast(1.05);
  box-shadow: 0 12px 28px rgba(0,0,0,0.12);
}
```

### 3.4 深色主题适配
- 使用 `:root[data-theme="dark"]` 覆盖背景/边框透明度
- 避免过亮背景导致低对比度

---

## 4. 分阶段改造计划

### 阶段 1：Shortcut 子集匹配逻辑（0.5 天）
1. 新增 `tokenizeQuery` / `isTokenSubset` 逻辑
2. Shortcut 模式下用子集匹配替代严格匹配
3. 保持“只允许 name 命中”规则

**验收**：
- `neovim close` → 命中 `neovim-close-tab`
- `git checkout` → 命中 `git-checkout`

---

### 阶段 2：New Page 操作按钮重排（0.5 天）
1. 移除顶部 Add Example 按钮
2. 新增 `insertExample(index)`
3. 每条示例右侧渲染 `+ / ×` 按钮

**验收**：
- 每条示例右侧可插入/删除
- 无需滚动到顶部

---

### 阶段 3：Append 模式限制（0.2 天）
1. `newMode === 'append'` 时仅显示第一个示例
2. 隐藏插入/删除按钮

**验收**：
- Append 模式只有一个示例框
- 插入/删除按钮不可见

---

### 阶段 4：示例卡片玻璃 UI（0.5 天）
1. `.example-row` 改为玻璃拟态卡片
2. Hover 时增加液体玻璃效果
3. Dark mode 适配

**验收**：
- 示例之间视觉分割清晰
- hover 有轻微浮起/滤镜变化

---

### 阶段 5：回归与测试（0.3 天）
1. 快捷键搜索子集匹配验证
2. New Page 插入/删除逻辑验证
3. Append 模式逻辑验证
4. UI 样式在暗色/亮色主题检查

**验收**：
- 无功能回归
- UI/交互满足需求

---

## 5. 风险与注意事项

| 风险 | 说明 | 缓解 |
|------|------|------|
| 子集匹配误命中 | 多个候选页匹配 | 取第一个或按字典排序 |
| Glass UI 兼容性 | 不支持 blur 的浏览器 | 提供 fallback 背景色 |
| Append 模式误操作 | 用户误以为可插入 | 隐藏按钮并提示 |

---

## 6. 结论

该方案能够解决 Shortcut 模式模糊搜索缺失问题，同时提升 New Page 示例交互体验，并用液体玻璃卡片增强可读性。所有变更集中在前端，风险可控，且不影响核心数据结构。
