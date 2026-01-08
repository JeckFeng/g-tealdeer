#!/bin/bash
set -e

PHASE=$1

if [ -z "$PHASE" ]; then
    echo "用法: ./execute_phase.sh <阶段号>"
    echo "阶段: 0, 1, 2, 3, 4, 5, 6"
    exit 1
fi

case $PHASE in
    0)
        echo "=== 阶段 0：准备工作 ==="
        git checkout -b workspace-refactor || git checkout workspace-refactor
        git add -A
        git commit -m "Checkpoint: Before workspace refactor" || echo "Already committed"
        
        cp -r src src.backup
        cp Cargo.toml Cargo.toml.backup
        
        echo "✓ 阶段 0 完成"
        ;;
    
    1)
        echo "=== 阶段 1：创建 Workspace 结构 ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 1: Workspace structure'"
        ;;
    
    2)
        echo "=== 阶段 2：提取核心库 ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 2: Core library'"
        ;;
    
    3)
        echo "=== 阶段 3：创建独立 CLI ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 3: Standalone CLI'"
        ;;
    
    4)
        echo "=== 阶段 4：修改 GUI ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 4: GUI refactor'"
        ;;
    
    5)
        echo "=== 阶段 5：清理 ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 5: Cleanup'"
        ;;
    
    6)
        echo "=== 阶段 6：打包 ==="
        echo "请手动执行 WORKSPACE_PHASED_PLAN.md 中的步骤"
        echo "完成后运行: git add -A && git commit -m 'Phase 6: Packaging'"
        ;;
    
    *)
        echo "无效的阶段号: $PHASE"
        exit 1
        ;;
esac
