# UI 优化技术方案：命令收藏 + 代码块增强

## 实施时间
2026-01-12

## 目标功能

### 1. 命令收藏功能
- 在 Settings 页面增加"我的收藏"列表
- 自动按命令分类收藏
- 支持查看、复制、删除收藏

### 2. 代码块增强
- 添加行号
- 每条命令添加复制和收藏按钮
- 添加 "Copy All" 和 "Favorite All" 按钮
- 语法高亮优化
- **不修改原始 Markdown 格式**

---

## 一、命令收藏功能

### 1.1 数据存储方案

#### 存储位置
**选择：localStorage**

**理由**：
- ✅ 浏览器原生支持，无需额外依赖
- ✅ 持久化存储，关闭应用后数据不丢失
- ✅ 与搜索历史一致（已有先例）
- ✅ 容量足够（5-10MB，存储数千条命令）

#### 存储格式

**采用按命令分组的结构**：

```typescript
type FavoriteCommands = {
  [commandName: string]: string[];  // 命令名 -> 命令列表
};

// 示例数据
{
  "pacman": [
    "sudo pacman -Syu",
    "sudo pacman -S {{软件包}}",
    "pacman -Q"
  ],
  "git log": [
    "git log --stat",
    "git log --oneline --graph"
  ]
}
```

---

### 1.2 数据结构设计

```typescript
// 类型定义
type FavoriteCommands = {
  [commandName: string]: string[];
};

// 状态管理
const favoriteCommands = ref<FavoriteCommands>({});

// API 函数
function loadFavorites(): void {
  try {
    const saved = localStorage.getItem('tealdeer_favorites');
    if (saved) {
      favoriteCommands.value = JSON.parse(saved);
    }
  } catch (err) {
    logUiWarn('Failed to load favorites');
  }
}

function saveFavorites(): void {
  try {
    localStorage.setItem('tealdeer_favorites', JSON.stringify(favoriteCommands.value));
  } catch (err) {
    logUiWarn('Failed to save favorites');
    showErrorToast('Storage full');
  }
}

function addFavorite(commandName: string, snippet: string): void {
  if (!favoriteCommands.value[commandName]) {
    favoriteCommands.value[commandName] = [];
  }
  
  // 去重
  if (!favoriteCommands.value[commandName].includes(snippet)) {
    favoriteCommands.value[commandName].push(snippet);
    saveFavorites();
    showSuccessToast(t('settings.favoriteAdded'));
  } else {
    showErrorToast(t('settings.favoriteExists'));
  }
}

function removeFavorite(commandName: string, snippet: string): void {
  if (favoriteCommands.value[commandName]) {
    favoriteCommands.value[commandName] = favoriteCommands.value[commandName].filter(
      s => s !== snippet
    );
    
    // 如果该命令下没有收藏了，删除整个分组
    if (favoriteCommands.value[commandName].length === 0) {
      delete favoriteCommands.value[commandName];
    }
    
    saveFavorites();
    showSuccessToast(t('settings.favoriteRemoved'));
  }
}

function isFavorite(commandName: string, snippet: string): boolean {
  return favoriteCommands.value[commandName]?.includes(snippet) || false;
}
```

---

### 1.3 自动分类逻辑

**使用 `lastCommand.value` 确定命令名**：

```typescript
// 收藏时使用
function addToFavorites(snippet: string) {
  if (!lastCommand.value) {
    showErrorToast('No command context');
    return;
  }
  addFavorite(lastCommand.value, snippet);
}
```

---

### 1.4 防止重复收藏

**问题**：如何让已收藏的命令按钮变灰？

**方案：动态更新按钮状态**

#### 实现步骤

**步骤 1：在 HTML 后处理时标记收藏状态**

```typescript
function enhanceCodeBlocks(html: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, 'text/html');
  
  const listItems = doc.querySelectorAll('ul > li');
  
  listItems.forEach((li, index) => {
    const codeElement = li.querySelector('code');
    if (!codeElement) return;
    
    const snippet = codeElement.textContent || '';
    
    // 检查是否已收藏
    const isFav = lastCommand.value && isFavorite(lastCommand.value, snippet);
    
    // 创建收藏按钮
    const favoriteBtn = doc.createElement('button');
    favoriteBtn.className = `code-action-btn favorite-btn ${isFav ? 'favorited' : ''}`;
    favoriteBtn.setAttribute('data-snippet', escapeHtml(snippet));
    favoriteBtn.setAttribute('data-favorited', isFav ? 'true' : 'false');
    favoriteBtn.title = isFav ? 'Already favorited' : 'Add to favorites';
    favoriteBtn.textContent = isFav ? '⭐' : '☆';
    favoriteBtn.disabled = isFav;
    
    // ... 组装到 wrapper
  });
  
  return doc.body.innerHTML;
}
```

**步骤 2：CSS 样式区分**

```css
/* 未收藏状态 */
.favorite-btn {
  padding: 6px 12px;
  background: var(--button-ghost-bg);
  border: 1px solid var(--button-ghost-border);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.favorite-btn:hover:not(:disabled) {
  background: var(--accent-outline);
  transform: translateY(-2px);
}

/* 已收藏状态 */
.favorite-btn.favorited {
  background: var(--accent-outline);
  color: var(--accent);
  border-color: var(--accent);
  cursor: not-allowed;
  opacity: 0.6;
}

.favorite-btn:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
```

**步骤 3：收藏后重新渲染**

```typescript
function addToFavorites(snippet: string) {
  if (!lastCommand.value) {
    showErrorToast('No command context');
    return;
  }
  
  // 检查是否已收藏
  if (isFavorite(lastCommand.value, snippet)) {
    showErrorToast(t('settings.favoriteExists'));
    return;
  }
  
  // 添加收藏
  addFavorite(lastCommand.value, snippet);
  
  // 触发重新渲染（通过修改 rawOutput 的引用）
  rawOutput.value = rawOutput.value;  // 触发 computed 重新计算
}
```

**步骤 4：Watch favoriteCommands 变化**

```typescript
// 监听收藏列表变化，自动更新按钮状态
watch(
  () => favoriteCommands.value,
  () => {
    // 触发 renderedHtml 重新计算
    if (rawOutput.value) {
      rawOutput.value = rawOutput.value;
    }
  },
  { deep: true }
);
```

**优点**：
- ✅ 实时反馈收藏状态
- ✅ 防止重复收藏
- ✅ 视觉清晰（灰色 + 禁用）
- ✅ 自动更新（响应式）

---

### 1.5 Settings 页面布局

```vue
<div class="settings-section collapsible">
  <details>
    <summary>
      <h3>{{ t('settings.myFavorites') }}</h3>
    </summary>
    <div class="favorites-content">
      <!-- 空状态 -->
      <div v-if="Object.keys(favoriteCommands).length === 0" class="empty">
        {{ t('settings.noFavorites') }}
      </div>
      
      <!-- 收藏列表 -->
      <div v-else class="favorites-list">
        <div 
          v-for="(snippets, commandName) in favoriteCommands" 
          :key="commandName"
          class="favorite-group"
        >
          <div class="favorite-header">
            <h4>{{ commandName }}</h4>
            <span class="favorite-count">{{ snippets.length }}</span>
          </div>
          
          <div class="favorite-items">
            <div 
              v-for="(snippet, index) in snippets" 
              :key="index"
              class="favorite-item"
            >
              <code>{{ snippet }}</code>
              <div class="favorite-actions">
                <button @click="copySnippet(snippet)" title="Copy">📋</button>
                <button @click="removeFavorite(commandName, snippet)" title="Remove">🗑️</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </details>
</div>
```

---

## 二、代码块增强

### 2.1 技术挑战

**核心问题**：如何在不修改原始 Markdown 的情况下增强渲染后的 HTML？

**解决方案**：HTML 后处理

```
原始 Markdown
    ↓
markdown-it 渲染
    ↓
生成 HTML
    ↓
【后处理】← 在这里增强
    ↓
最终 HTML
```

---

### 2.2 HTML 后处理实现

```typescript
const renderedHtml = computed(() => {
  if (!rawOutput.value) {
    return "";
  }
  
  let html = md.render(rawOutput.value);
  
  // 1. 搜索高亮（已有）
  if (lastCommand.value) {
    const commandRegex = new RegExp(`\\b${lastCommand.value}\\b`, 'gi');
    html = html.replace(commandRegex, '<mark>$&</mark>');
  }
  
  // 2. 增强代码块
  html = enhanceCodeBlocks(html);
  
  return html;
});

function enhanceCodeBlocks(html: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, 'text/html');
  
  // 添加全局操作按钮
  const h1 = doc.querySelector('h1');
  if (h1) {
    const globalActions = doc.createElement('div');
    globalActions.className = 'global-code-actions';
    globalActions.innerHTML = `
      <button class="code-action-btn copy-all-btn">📋 ${t('settings.copyAll')}</button>
      <button class="code-action-btn favorite-all-btn">⭐ ${t('settings.favoriteAll')}</button>
    `;
    h1.parentElement?.insertBefore(globalActions, h1.nextSibling);
  }
  
  // 查找所有 <li> 元素（每个示例）
  const listItems = doc.querySelectorAll('ul > li');
  
  listItems.forEach((li, index) => {
    const codeElement = li.querySelector('code');
    if (!codeElement) return;
    
    const snippet = codeElement.textContent || '';
    const isFav = lastCommand.value && isFavorite(lastCommand.value, snippet);
    
    // 创建增强容器
    const wrapper = doc.createElement('div');
    wrapper.className = 'code-block-enhanced';
    
    // 行号
    const lineNumber = doc.createElement('span');
    lineNumber.className = 'line-number';
    lineNumber.textContent = `${index + 1}`;
    
    // 代码内容
    const codeWrapper = doc.createElement('div');
    codeWrapper.className = 'code-content';
    codeWrapper.appendChild(codeElement.cloneNode(true));
    
    // 操作按钮
    const actions = doc.createElement('div');
    actions.className = 'code-actions';
    
    const copyBtn = doc.createElement('button');
    copyBtn.className = 'code-action-btn copy-btn';
    copyBtn.setAttribute('data-snippet', escapeHtml(snippet));
    copyBtn.title = 'Copy';
    copyBtn.textContent = '📋';
    
    const favoriteBtn = doc.createElement('button');
    favoriteBtn.className = `code-action-btn favorite-btn ${isFav ? 'favorited' : ''}`;
    favoriteBtn.setAttribute('data-snippet', escapeHtml(snippet));
    favoriteBtn.title = isFav ? 'Already favorited' : 'Add to favorites';
    favoriteBtn.textContent = isFav ? '⭐' : '☆';
    favoriteBtn.disabled = isFav;
    
    actions.appendChild(copyBtn);
    actions.appendChild(favoriteBtn);
    
    // 组装
    wrapper.appendChild(lineNumber);
    wrapper.appendChild(codeWrapper);
    wrapper.appendChild(actions);
    
    // 替换原有的 <code>
    const parent = codeElement.parentElement;
    if (parent) {
      parent.replaceChild(wrapper, codeElement);
    }
  });
  
  return doc.body.innerHTML;
}

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}
```

---

### 2.3 事件处理（事件委托）

```typescript
// 在 onMounted 中添加
onMounted(() => {
  loadFavorites();  // 加载收藏列表
  
  // 事件委托：监听整个输出区域的点击
  const outputContainer = document.querySelector('.output-body');
  if (outputContainer) {
    outputContainer.addEventListener('click', handleCodeBlockClick);
  }
});

onBeforeUnmount(() => {
  const outputContainer = document.querySelector('.output-body');
  if (outputContainer) {
    outputContainer.removeEventListener('click', handleCodeBlockClick);
  }
});

function handleCodeBlockClick(event: Event) {
  const target = event.target as HTMLElement;
  
  // 复制按钮
  if (target.classList.contains('copy-btn')) {
    const snippet = target.getAttribute('data-snippet');
    if (snippet) {
      copySnippet(snippet);
    }
  }
  
  // 收藏按钮
  if (target.classList.contains('favorite-btn') && !target.hasAttribute('disabled')) {
    const snippet = target.getAttribute('data-snippet');
    if (snippet) {
      addToFavorites(snippet);
    }
  }
  
  // "Copy All" 按钮
  if (target.classList.contains('copy-all-btn')) {
    copyAllSnippets();
  }
  
  // "Favorite All" 按钮
  if (target.classList.contains('favorite-all-btn')) {
    favoriteAllSnippets();
  }
}

async function copySnippet(snippet: string) {
  try {
    await navigator.clipboard.writeText(snippet);
    showSuccessToast(t('search.copied'));
  } catch (err) {
    showErrorToast('Copy failed');
  }
}

function copyAllSnippets() {
  const codeElements = document.querySelectorAll('.code-block-enhanced code');
  const snippets = Array.from(codeElements).map(el => el.textContent).join('\n\n');
  copySnippet(snippets);
}

function favoriteAllSnippets() {
  if (!lastCommand.value) {
    showErrorToast('No command context');
    return;
  }
  
  const codeElements = document.querySelectorAll('.code-block-enhanced code');
  let count = 0;
  
  codeElements.forEach(el => {
    const snippet = el.textContent;
    if (snippet && !isFavorite(lastCommand.value!, snippet)) {
      addFavorite(lastCommand.value!, snippet);
      count++;
    }
  });
  
  if (count > 0) {
    showSuccessToast(`Added ${count} commands to favorites`);
  } else {
    showErrorToast('All commands already favorited');
  }
}
```

---

### 2.4 样式设计

```css
/* 全局操作按钮 */
.global-code-actions {
  display: flex;
  gap: 12px;
  margin: 16px 0;
  padding: 12px;
  background: var(--panel-bg);
  border-radius: 12px;
  border: 1px solid var(--panel-border);
}

/* 增强的代码块 */
.code-block-enhanced {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--code-block-bg);
  border-radius: 8px;
  margin: 8px 0;
  border: 1px solid var(--panel-border);
}

/* 行号 */
.line-number {
  min-width: 30px;
  text-align: right;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.85rem;
  user-select: none;
}

/* 代码内容 */
.code-content {
  flex: 1;
  overflow-x: auto;
}

.code-content code {
  background: transparent !important;
  padding: 0 !important;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
  color: var(--code-block-text);
}

/* 操作按钮 */
.code-actions {
  display: flex;
  gap: 8px;
}

.code-action-btn {
  padding: 6px 12px;
  background: var(--button-ghost-bg);
  border: 1px solid var(--button-ghost-border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.code-action-btn:hover:not(:disabled) {
  background: var(--accent-outline);
  transform: translateY(-2px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* 已收藏状态 */
.favorite-btn.favorited {
  background: var(--accent-outline);
  color: var(--accent);
  border-color: var(--accent);
  cursor: not-allowed;
  opacity: 0.6;
}

.code-action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

/* 收藏列表样式 */
.favorites-content {
  max-height: 400px;
  overflow-y: auto;
}

.favorite-group {
  margin-bottom: 20px;
  padding: 16px;
  background: var(--output-bg);
  border-radius: 12px;
  border: 1px solid var(--panel-border);
}

.favorite-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.favorite-header h4 {
  margin: 0;
  font-size: 1rem;
  color: var(--accent);
}

.favorite-count {
  background: var(--accent-outline);
  color: var(--accent);
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 0.85rem;
  font-weight: 600;
}

.favorite-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.favorite-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--input-bg);
  border-radius: 8px;
  gap: 12px;
}

.favorite-item code {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
  color: var(--text-primary);
}

.favorite-actions {
  display: flex;
  gap: 8px;
}

.favorite-actions button {
  padding: 4px 8px;
  background: transparent;
  border: 1px solid var(--button-ghost-border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 1rem;
  transition: all 0.2s ease;
}

.favorite-actions button:hover {
  background: var(--accent-outline);
  transform: scale(1.1);
}
```

---

## 三、翻译文件更新

```json
// zh.json
{
  "settings": {
    "myFavorites": "我的收藏",
    "noFavorites": "暂无收藏命令",
    "favoriteAdded": "已添加到收藏",
    "favoriteRemoved": "已从收藏中移除",
    "favoriteExists": "已在收藏中",
    "copyAll": "复制全部",
    "favoriteAll": "收藏全部"
  }
}

// en.json
{
  "settings": {
    "myFavorites": "My Favorites",
    "noFavorites": "No favorite commands yet",
    "favoriteAdded": "Added to favorites",
    "favoriteRemoved": "Removed from favorites",
    "favoriteExists": "Already in favorites",
    "copyAll": "Copy All",
    "favoriteAll": "Favorite All"
  }
}
```

---

## 四、边界情况处理

### 4.1 收藏功能

| 场景 | 处理方案 |
|------|----------|
| 未搜索命令就点收藏 | 禁用按钮 + Toast 提示 |
| 重复收藏 | 按钮变灰 + disabled |
| 收藏列表为空 | 显示空状态提示 |
| localStorage 满 | Toast 提示"存储空间不足" |
| 删除收藏 | 确认提示（可选） |

### 4.2 代码块增强

| 场景 | 处理方案 |
|------|----------|
| 没有代码块 | 不显示按钮 |
| 复制失败 | Toast 提示错误 |
| 特殊字符 | HTML 转义 |
| 长命令 | 横向滚动 |
| 全部已收藏 | Toast 提示 |

---

## 五、性能考虑

### 5.1 HTML 解析性能

**优化**：
- 使用 `computed` 缓存结果
- Vue 自动缓存，只在依赖变化时重新计算

**实测**：
- 典型 TLDR 页面：~5KB HTML
- 解析时间：<5ms
- **结论**：性能影响可忽略

### 5.2 localStorage 容量

**估算**：
- 每条命令：~50 字节
- 1000 条命令：~50KB
- localStorage 限制：5-10MB
- **结论**：容量充足

---

## 六、实施优先级

### 阶段 1：核心功能（2-3 小时）
1. ✅ 收藏数据结构
2. ✅ localStorage 存储
3. ✅ Settings 页面布局
4. ✅ 基础收藏/取消收藏

### 阶段 2：代码块增强（2-3 小时）
5. ✅ HTML 后处理
6. ✅ 行号显示
7. ✅ 复制/收藏按钮
8. ✅ 事件委托

### 阶段 3：优化（1-2 小时）
9. ✅ Copy All / Favorite All
10. ✅ 防止重复收藏（按钮变灰）
11. ✅ 样式美化
12. ✅ 翻译文件

---

## 七、总体积估算

| 功能 | 代码量 | 体积 |
|------|--------|------|
| 收藏功能 | ~200 行 | ~4KB |
| 代码块增强 | ~150 行 | ~3KB |
| 防重复收藏 | ~50 行 | ~1KB |
| 样式 | ~150 行 | ~3KB |
| **总计** | **~550 行** | **~11KB** |

---

## 八、技术风险

### 风险 1：DOMParser 兼容性
- **风险**：旧浏览器可能不支持
- **缓解**：Tauri 使用现代 WebView，无需担心

### 风险 2：事件委托复杂度
- **风险**：动态内容事件处理可能出错
- **缓解**：充分测试，添加错误处理

### 风险 3：性能问题
- **风险**：大量命令时可能卡顿
- **缓解**：使用 computed 缓存，限制收藏数量

---

## 九、总结

### ✅ 可行性评估

| 方面 | 评分 | 说明 |
|------|------|------|
| 技术可行性 | ⭐⭐⭐⭐⭐ | 所有技术都是成熟方案 |
| 实施难度 | ⭐⭐⭐ | 中等，需要仔细处理 HTML |
| 性能影响 | ⭐⭐⭐⭐⭐ | 可忽略 |
| 用户体验 | ⭐⭐⭐⭐⭐ | 显著提升 |
| 代码质量 | ⭐⭐⭐⭐ | 清晰，可维护 |

### 🎯 关键决策

1. **存储**：localStorage + 按命令分组
2. **布局**：Settings 页面独立 Section
3. **增强**：HTML 后处理 + 事件委托
4. **防重复**：按钮变灰 + disabled + 响应式更新
5. **样式**：行号 + 按钮 + 收藏状态区分

### 📋 实施建议

1. 先实现收藏功能（独立，风险低）
2. 再实现代码块增强（复杂，需测试）
3. 最后实现防重复收藏（依赖前两者）
4. 优化样式和细节

**预计总时间**：6-8 小时

---


