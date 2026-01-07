# TealDeer-Tile 国际化实施指南

## 已完成的工作 ✅

### 1. 安装依赖
```bash
npm install vue-i18n@latest
```

### 2. 创建翻译文件
- ✅ `src/locales/en.json` - 英文翻译
- ✅ `src/locales/zh.json` - 中文翻译  
- ✅ `src/locales/index.ts` - i18n配置

### 3. 注册i18n插件
- ✅ 修改 `src/main.ts` 注册i18n
- ✅ 在 `App.vue` 中导入 `useI18n`
- ✅ 添加语言切换监听和保存

## 下一步：修改模板使用翻译

### 模板修改示例

#### 原代码：
```vue
<h1>Tealdeer-Tile</h1>
<p class="subtitle">
  Write, manage, search—your tealdeer pages, your way.
</p>
```

#### 修改后：
```vue
<h1>{{ t('app.title') }}</h1>
<p class="subtitle">
  {{ t('app.subtitle') }}
</p>
```

### 需要修改的主要区域

#### 1. Header (品牌区域)
```vue
<!-- 原代码 -->
<h1>Tealdeer-Tile</h1>
<p class="subtitle">Write, manage, search—your tealdeer pages, your way.</p>

<!-- 修改为 -->
<h1>{{ t('app.title') }}</h1>
<p class="subtitle">{{ t('app.subtitle') }}</p>
```

#### 2. Theme Toggle (主题切换)
```vue
<!-- 原代码 -->
<span class="theme-label">Theme: {{ themeToggleLabel }}</span>

<!-- 修改为 -->
<span class="theme-label">{{ t('theme.label') }}: {{ themeToggleLabel }}</span>
```

#### 3. Tabs (标签页)
```vue
<!-- 原代码 -->
<button>Search</button>
<button>New Page</button>
<button>Manage</button>
<button>Settings</button>

<!-- 修改为 -->
<button>{{ t('tabs.search') }}</button>
<button>{{ t('tabs.newPage') }}</button>
<button>{{ t('tabs.manage') }}</button>
<button>{{ t('tabs.settings') }}</button>
```

#### 4. Settings页面添加语言选择器
```vue
<div class="settings-fields">
  <!-- 在 Display & Interaction 部分添加 -->
  <label class="field">
    <span>{{ t('settings.interfaceLanguage') }}</span>
    <select v-model="locale">
      <option value="en">English</option>
      <option value="zh">中文</option>
    </select>
  </label>
  
  <!-- 其他选项... -->
</div>
```

## 快速实施方案

由于完整修改所有文本需要大量时间，建议采用**渐进式实施**：

### 阶段1: 核心功能（30分钟）
- ✅ 已完成：安装和配置i18n
- ⏳ 待完成：修改主要UI文本（标题、标签页、按钮）
- ⏳ 待完成：添加语言切换器

### 阶段2: 详细内容（1-2小时）
- Settings页面所有选项
- Search页面所有文本
- New Page页面所有文本
- Manage页面所有文本

### 阶段3: 提示和消息（30分钟）
- 成功/错误消息
- 提示文本
- 占位符文本

## 自动化脚本建议

可以使用正则表达式批量替换常见模式：

```bash
# 替换简单文本
sed -i 's/>Search</>{{ t("tabs.search") }}</g' src/App.vue
sed -i 's/>Settings</>{{ t("tabs.settings") }}</g' src/App.vue

# 但需要手动检查和调整
```

## 测试清单

- [ ] 英文界面显示正常
- [ ] 中文界面显示正常
- [ ] 语言切换实时生效
- [ ] 语言偏好持久化保存
- [ ] 所有页面文本已翻译
- [ ] 布局在两种语言下都正常
- [ ] 按钮和输入框大小适配

## 当前状态

**基础设施**: ✅ 100% 完成
**模板修改**: ⏳ 0% 完成（需要手动修改约200-300处文本）

## 建议

由于模板修改工作量较大，建议：

1. **优先级1**: 修改用户最常看到的文本（标题、标签页、主要按钮）
2. **优先级2**: 修改Settings页面（用户会仔细阅读）
3. **优先级3**: 修改其他页面的详细文本

或者，如果时间有限，可以：
- 保持当前英文界面
- 仅添加语言切换器作为未来功能的准备
- 逐步翻译，每次发布增加一些翻译内容
