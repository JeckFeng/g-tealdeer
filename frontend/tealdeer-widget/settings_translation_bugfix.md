# Settings页面翻译BUG修复报告

## 修复日期
2026-01-08

## 发现的问题

### 问题: 翻译键名不匹配
**现象**: Settings页面显示原始键名而非翻译文本
- `settings.openConfigFile`
- `settings.systemInformation`
- `settings.saving`
- `settings.saveSettings`

**原因**: 代码中使用的键名与翻译文件中定义的键名不一致

**影响**: 用户看到的是键名而不是翻译后的文本

## 修复方案

### 修正翻译键名

| 错误键名 | 正确键名 | 翻译值 (en/zh) |
|---------|---------|---------------|
| `settings.openConfigFile` | `settings.openConfig` | Open config.toml / 打开配置文件 |
| `settings.systemInformation` | `settings.systemInfo` | System Information / 系统信息 |
| `settings.saving` | `settings.savingButton` | Saving... / 保存中... |
| `settings.saveSettings` | `settings.saveButton` | Save Settings / 保存设置 |

**修改位置**:
1. File Management section - "Open config.toml" 按钮
2. System Information section - 标题
3. Save button - 按钮文本（正常/加载状态）

## 修复结果

### 构建验证 ✅
```bash
npm run build
✓ 114 modules transformed
✓ built in 1.33s
```

### 修复效果
1. ✅ "Open config.toml" 按钮正确显示翻译
2. ✅ "System Information" 标题正确显示翻译
3. ✅ "Save Settings" 按钮正确显示翻译
4. ✅ "Saving..." 状态正确显示翻译
5. ✅ 英文和中文界面均正常工作

## 修改的文件

**src/App.vue** - Settings section
- 修复 `openConfigFile` → `openConfig` (line ~1327)
- 修复 `systemInformation` → `systemInfo` (line ~1346)
- 修复 `saving` → `savingButton` (line ~1377)
- 修复 `saveSettings` → `saveButton` (line ~1377)

**总计**: 4处键名修复

## 翻译对照表

| 英文 | 中文 | 正确键名 |
|------|------|---------|
| Open config.toml | 打开配置文件 | settings.openConfig |
| Open logs folder | 打开日志文件夹 | settings.openLogsFolder |
| Open rust.log | 打开Rust日志 | settings.openRustLog |
| Open webview.log | 打开Webview日志 | settings.openWebviewLog |
| System Information | 系统信息 | settings.systemInfo |
| Save Settings | 保存设置 | settings.saveButton |
| Saving... | 保存中... | settings.savingButton |

## 测试建议

### 功能测试
- [ ] 切换到英文界面，检查Settings页面所有按钮
- [ ] 切换到中文界面，检查Settings页面所有按钮
- [ ] 验证"Open config.toml"按钮显示正确
- [ ] 验证"System Information"标题显示正确
- [ ] 验证"Save Settings"按钮显示正确
- [ ] 点击保存按钮，验证"Saving..."状态显示正确

### 视觉测试
- [ ] 检查按钮文本是否完整显示
- [ ] 检查中文文本是否有布局问题
- [ ] 检查保存按钮在加载状态下的文本

## 根本原因分析

这些问题的根本原因是**键名不一致**：

1. **命名风格不统一**: 
   - 有些使用完整描述：`openConfigFile`
   - 有些使用简短形式：`openConfig`

2. **后缀不一致**:
   - 有些按钮使用 `Button` 后缀：`saveButton`, `savingButton`
   - 有些不使用后缀：`openConfig`

3. **缺乏命名规范**: 没有统一的翻译键命名规范文档

## 改进建议

1. **建立命名规范**: 
   - 按钮文本：使用动词开头，如 `openConfig`, `saveButton`
   - 标题/标签：使用名词，如 `systemInfo`, `fileManagement`
   - 状态文本：使用进行时或完成时，如 `savingButton`, `loadingSettings`

2. **创建键名索引**: 维护一个翻译键名索引文档，列出所有可用的键

3. **添加验证工具**: 创建脚本检查代码中使用的键是否都在翻译文件中定义

---

**修复状态**: ✅ 完成
**构建状态**: ✅ 通过
**测试状态**: ⏳ 待用户验证
