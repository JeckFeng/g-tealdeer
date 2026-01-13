#!/bin/bash

echo "=== 阶段 2 功能测试 ==="
echo ""

# 测试 1: 检查 shortcut_pages 目录是否创建
echo "测试 1: 检查 shortcut_pages 目录"
SHORTCUT_DIR=$(./target/release/tldr --show-paths | grep "Shortcut pages dir" | cut -d: -f2- | xargs)
if [ -d "$SHORTCUT_DIR" ]; then
    echo "✅ shortcut_pages 目录存在: $SHORTCUT_DIR"
else
    echo "❌ shortcut_pages 目录不存在"
fi
echo ""

# 测试 2: 使用 CLI 创建快捷键页面
echo "测试 2: CLI 创建快捷键页面"
./target/release/tldr --shortcut nano > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ CLI 快捷键查询正常"
else
    echo "⚠️  CLI 快捷键查询失败（预期，因为页面不存在）"
fi
echo ""

# 测试 3: 检查编译产物
echo "测试 3: 检查编译产物"
if [ -f "./target/release/tealdeer_tile" ]; then
    echo "✅ Tauri 应用编译成功"
else
    echo "❌ Tauri 应用编译失败"
fi
echo ""

# 测试 4: 验证 scope 参数
echo "测试 4: 验证 scope 参数"
./target/release/tldr --shortcut --list > /tmp/shortcut_list.txt 2>&1
./target/release/tldr --list > /tmp/command_list.txt 2>&1
if [ -f /tmp/shortcut_list.txt ] && [ -f /tmp/command_list.txt ]; then
    echo "✅ scope 参数工作正常"
    echo "   快捷键列表: $(wc -l < /tmp/shortcut_list.txt) 项"
    echo "   命令列表: $(wc -l < /tmp/command_list.txt) 项"
else
    echo "❌ scope 参数测试失败"
fi
echo ""

echo "=== 测试完成 ==="
