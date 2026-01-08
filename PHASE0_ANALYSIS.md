# 阶段 0 分析报告

## 代码结构分析

### 源文件列表
```
src/
├── api.rs          (18.6 KB) - API 调用和主要逻辑
├── cache.rs        (14.7 KB) - 缓存管理
├── cli.rs          (3.3 KB)  - CLI 参数解析
├── config.rs       (26.6 KB) - 配置管理
├── extensions.rs   (0.9 KB)  - 扩展功能
├── formatter.rs    (8.4 KB)  - 页面渲染
├── lib.rs          (1.0 KB)  - 库入口
├── line_iterator.rs (3.9 KB) - 行迭代器
├── main.rs         (1.7 KB)  - CLI 入口
├── output.rs       (3.6 KB)  - 输出处理
├── types.rs        (7.1 KB)  - 数据类型
└── utils.rs        (0.7 KB)  - 工具函数
```

### 依赖关系
- anyhow: 错误处理
- app_dirs2: 应用目录
- clap: CLI 参数解析
- log: 日志
- serde: 序列化
- toml: 配置文件
- ureq: HTTP 请求
- yansi: 颜色输出
- zip: 压缩文件

### lib.rs 导出
```rust
pub mod cli;
pub mod types;
pub mod api;
pub mod extensions;
pub use api::{compute_enable_styles, make_seed_config, run, RunArgs, RunOutput, ShowPaths};
pub use cli::Cli;
```

## 核心模块识别

### 不依赖 CLI 的核心模块（可移到 tealdeer-core）
- ✅ cache.rs - 缓存管理
- ✅ config.rs - 配置管理
- ✅ formatter.rs - 页面渲染
- ✅ utils.rs - 工具函数
- ✅ api.rs - API 调用
- ✅ line_iterator.rs - 行迭代器
- ✅ output.rs - 输出处理
- ✅ extensions.rs - 扩展功能
- ⚠️ types.rs - 数据类型（需要移除 clap 依赖）

### CLI 特定模块（保留在 CLI 包）
- ❌ cli.rs - CLI 参数解析（依赖 clap）
- ❌ main.rs - CLI 入口

### lib.rs 处理
- 需要重新设计，分离核心库和 CLI 库

## 关键问题

### 1. types.rs 的 clap 依赖
```rust
// types.rs 中使用了 clap::ValueEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PlatformType {
    Linux,
    MacOS,
    Windows,
    Common,
}
```

**解决方案**：
- 在 tealdeer-core 中移除 clap::ValueEnum
- 添加 from_str() 方法手动解析
- CLI 包中可以重新实现 ValueEnum

### 2. 模块间依赖
- api.rs 可能依赖 cli.rs 的某些类型
- 需要仔细检查每个模块的导入

### 3. 特性标志
- 现有代码使用了 TLS 特性标志
- 需要在 workspace 中正确配置

## 下一步计划

### 阶段 1：创建 Workspace 结构
1. 创建 tealdeer-legacy 目录
2. 移动现有代码到 tealdeer-legacy
3. 创建 Workspace Cargo.toml
4. 确保 tealdeer-legacy 可以编译

### 阶段 2：提取核心库
1. 创建 tealdeer-core
2. 复制核心模块
3. 修改 types.rs 移除 clap 依赖
4. 创建 lib.rs 导出核心 API

## 备份信息

- 备份目录：src.backup.phase0
- 备份配置：Cargo.toml.backup.phase0
- Git 分支：workspace-refactor
- Git 提交：de3305b

## 验收标准

- ✅ Git 分支已创建
- ✅ 代码已备份
- ✅ 依赖关系已记录
- ✅ 核心模块已识别
- ✅ 关键问题已标记

## 阶段 0 完成时间

2026-01-08 23:31
