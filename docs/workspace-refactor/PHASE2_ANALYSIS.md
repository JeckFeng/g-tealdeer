# 阶段 2 分析报告

## 尝试时间
2026-01-08 23:40 - 23:50

## 结论：❌ 提取核心库过于复杂，采用简化方案

---

## 遇到的问题

### 1. 模块间依赖复杂
- `api.rs` 依赖 `output.rs`（CLI 专用）
- `formatter.rs` 依赖 `extensions.rs`（CLI 专用）
- `config.rs` 依赖 `extensions::Dedup`
- 无法简单分离核心和 CLI 模块

### 2. clap 耦合严重
- `types.rs` 中 `PlatformType` 和 `ColorOptions` 使用 `clap::ValueEnum`
- 需要手动实现 `value_variants()` 方法
- 需要删除 `ValueEnum` trait 实现

### 3. TLS 配置复杂
- `cache.rs` 中 TLS 配置依赖 feature flags
- 类型推断失败（`TlsConfig::builder()` 返回类型不明确）
- 需要在编译时启用特定 feature

### 4. 工具函数缺失
- `extensions::Dedup` trait 需要重新实现或替换为标准库
- `clear_duplicates()` 需要替换为 `sort() + dedup()`
- `Language` 结构体字段访问需要修改

---

## 简化方案：直接复用 tealdeer-legacy

### 方案描述
- **不提取核心库**
- CLI 和 GUI 都直接依赖 `tealdeer-legacy`
- `tealdeer-legacy` 作为共享库使用

### 优点
1. ✅ 无需重构代码
2. ✅ 保持原有功能完整
3. ✅ 编译时间短
4. ✅ 维护成本低

### 缺点
1. ⚠️ CLI 和 GUI 都依赖 clap（但 GUI 不使用）
2. ⚠️ 包含一些 CLI 专用代码（如 pager）

### 实现步骤
1. 将 `tealdeer-legacy` 重命名为 `tealdeer-core`
2. 修改 `Cargo.toml` 配置为 library
3. 创建 `tldr` CLI 包，依赖 `tealdeer-core`
4. 创建 `tealdeer_tile` GUI 包，依赖 `tealdeer-core`

---

## 新的阶段 2 计划

### 2.1 重命名 tealdeer-legacy 为 tealdeer-core
```bash
mv tealdeer-legacy tealdeer-core
sed -i 's/tealdeer-legacy/tealdeer-core/g' Cargo.toml
```

### 2.2 修改 tealdeer-core/Cargo.toml
- 保持所有依赖（包括 clap）
- 配置为 library（移除 `[[bin]]`）
- 保留所有 features

### 2.3 验证编译
```bash
cargo build -p tealdeer-core
```

**预计时间**：10 分钟

---

## 对比：原方案 vs 简化方案

| 项目 | 原方案（提取核心库） | 简化方案（直接复用） |
|------|---------------------|---------------------|
| 代码重构 | 大量 | 无 |
| clap 依赖 | 移除 | 保留 |
| 编译时间 | 长 | 短 |
| 维护成本 | 高 | 低 |
| 功能完整性 | 可能丢失 | 100% |
| 实现时间 | 2-3 小时 | 10 分钟 |
| 包大小 | 更小 | 稍大 |

---

## 决策

**采用简化方案**

理由：
1. 原方案遇到的技术问题过多
2. 代码耦合度高，分离成本大
3. 简化方案能快速完成目标
4. GUI 多依赖 clap 的影响很小（约 100KB）

---

## 下一步

执行新的阶段 2：
1. 重命名 tealdeer-legacy → tealdeer-core
2. 修改 Cargo.toml 配置
3. 验证编译
4. 提交 Git

**预计完成时间**：23:55
