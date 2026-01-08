# Workspace 改造检查清单

## 使用方法

每完成一个阶段，在对应的复选框打勾。

---

## 阶段 0：准备工作 ⏱️ 30分钟

- [ ] 0.1 创建 Git 分支 `workspace-refactor`
- [ ] 0.2 分析现有代码结构
- [ ] 0.3 创建备份（src.backup, Cargo.toml.backup）
- [ ] 0.4 Git 提交：`git commit -m "Phase 0: Preparation"`

**验收**：
- [ ] Git 分支已创建
- [ ] 备份文件存在
- [ ] 可以回滚

---

## 阶段 1：创建 Workspace 结构 ⏱️ 1小时

- [ ] 1.1 创建 Workspace Cargo.toml
- [ ] 1.2 创建 tealdeer-legacy 目录
- [ ] 1.3 移动 src 到 tealdeer-legacy/
- [ ] 1.4 修改 tealdeer-legacy/Cargo.toml 使用 workspace 依赖
- [ ] 1.5 测试编译：`cargo build -p tealdeer-legacy`
- [ ] 1.6 功能测试：`./target/debug/tldr tar`
- [ ] 1.7 Git 提交：`git commit -m "Phase 1: Workspace structure"`

**验收**：
- [ ] `cargo build -p tealdeer-legacy` 成功
- [ ] `tldr tar` 功能正常
- [ ] 可以回滚

---

## 阶段 2：提取核心库 ⏱️ 3-4小时

- [ ] 2.1 创建 tealdeer-core 目录
- [ ] 2.2 创建 tealdeer-core/Cargo.toml
- [ ] 2.3 复制核心模块到 tealdeer-core/src/
  - [ ] cache.rs
  - [ ] config.rs
  - [ ] formatter.rs
  - [ ] types.rs（需要修改）
  - [ ] utils.rs
  - [ ] api.rs
  - [ ] line_iterator.rs
  - [ ] output.rs
  - [ ] extensions.rs
- [ ] 2.4 修改 types.rs 移除 clap 依赖
- [ ] 2.5 创建 tealdeer-core/src/lib.rs
- [ ] 2.6 更新 Workspace members
- [ ] 2.7 tealdeer-legacy 添加 tealdeer-core 依赖
- [ ] 2.8 修改 tealdeer-legacy 导入路径
- [ ] 2.9 测试编译：`cargo build -p tealdeer-core`
- [ ] 2.10 测试编译：`cargo build -p tealdeer-legacy`
- [ ] 2.11 功能测试：`./target/debug/tldr tar`
- [ ] 2.12 运行测试：`cargo test`
- [ ] 2.13 Git 提交：`git commit -m "Phase 2: Core library"`

**验收**：
- [ ] `cargo build -p tealdeer-core` 成功
- [ ] `cargo build -p tealdeer-legacy` 成功
- [ ] 所有功能正常
- [ ] 所有测试通过

---

## 阶段 3：创建独立 CLI ⏱️ 2小时

- [ ] 3.1 创建 tldr 目录
- [ ] 3.2 创建 tldr/Cargo.toml
- [ ] 3.3 复制 main.rs 和 cli.rs
- [ ] 3.4 修改导入路径
- [ ] 3.5 更新 Workspace members
- [ ] 3.6 测试编译：`cargo build --release -p tldr`
- [ ] 3.7 功能测试：
  - [ ] `./target/release/tldr tar`
  - [ ] `./target/release/tldr --update`
  - [ ] `./target/release/tldr --list`
- [ ] 3.8 检查二进制大小（应该 ~6MB）
- [ ] 3.9 检查启动速度（应该 ~10ms）
- [ ] 3.10 Git 提交：`git commit -m "Phase 3: Standalone CLI"`

**验收**：
- [ ] `cargo build -p tldr` 成功
- [ ] CLI 所有功能正常
- [ ] 性能符合预期

---

## 阶段 4：修改 GUI 使用核心库 ⏱️ 1-2小时

- [ ] 4.1 修改 GUI Cargo.toml 依赖
- [ ] 4.2 修改 GUI 源码导入路径
- [ ] 4.3 更新 Workspace members
- [ ] 4.4 测试编译：`npm run tauri -- build`
- [ ] 4.5 功能测试：
  - [ ] 启动 GUI
  - [ ] 搜索命令
  - [ ] 显示页面
  - [ ] 自定义页面
- [ ] 4.6 Git 提交：`git commit -m "Phase 4: GUI refactor"`

**验收**：
- [ ] GUI 编译成功
- [ ] GUI 所有功能正常

---

## 阶段 5：清理和优化 ⏱️ 1小时

- [ ] 5.1 删除 tealdeer-legacy
- [ ] 5.2 更新 Workspace members
- [ ] 5.3 更新 README.md
- [ ] 5.4 更新 Build_CLI.md
- [ ] 5.5 测试完整构建：`cargo build --release --workspace`
- [ ] 5.6 测试 GUI 构建：`npm run tauri -- build`
- [ ] 5.7 运行所有测试：`cargo test --workspace`
- [ ] 5.8 Git 提交：`git commit -m "Phase 5: Cleanup"`

**验收**：
- [ ] 所有包编译成功
- [ ] 所有测试通过
- [ ] 文档已更新

---

## 阶段 6：打包和发布 ⏱️ 1小时

- [ ] 6.1 构建 CLI：`cargo build --release -p tldr`
- [ ] 6.2 构建 GUI：`npm run tauri -- build`
- [ ] 6.3 安装 CLI：`cp target/release/tldr ~/.local/bin/`
- [ ] 6.4 安装 GUI：`cp ... ~/.local/bin/`
- [ ] 6.5 测试安装：
  - [ ] `tldr tar`
  - [ ] `tealdeer_tile`
- [ ] 6.6 检查文件大小
- [ ] 6.7 Git 提交：`git commit -m "Phase 6: Packaging"`
- [ ] 6.8 合并到主分支：`git checkout 1-8-4 && git merge workspace-refactor`

**验收**：
- [ ] CLI 和 GUI 都能正常运行
- [ ] 安装包大小合理
- [ ] 用户体验良好

---

## 最终验收

- [ ] CLI 独立运行（6MB，~10ms 启动）
- [ ] GUI 独立运行（18MB）
- [ ] 代码复用（核心库）
- [ ] 所有功能正常
- [ ] 所有测试通过
- [ ] 文档完整

---

## 回滚命令

```bash
# 回滚到上一个阶段
git reset --hard HEAD~1

# 回滚到开始
git checkout 1-8-4
git branch -D workspace-refactor
```

---

## 当前进度

**当前阶段**：阶段 0

**下一步**：执行 `./execute_phase.sh 0`
