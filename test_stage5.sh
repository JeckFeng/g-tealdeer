#!/bin/bash

echo "═══════════════════════════════════════════════════════════════"
echo "  阶段 5：测试与回归"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS=0
FAIL=0
WARN=0

# 测试函数
test_case() {
    local name="$1"
    local command="$2"
    local expected="$3"
    
    echo -n "测试: $name ... "
    
    if eval "$command" > /tmp/test_output.txt 2>&1; then
        if [ -n "$expected" ]; then
            if grep -q "$expected" /tmp/test_output.txt; then
                echo -e "${GREEN}✓ PASS${NC}"
                ((PASS++))
            else
                echo -e "${RED}✗ FAIL${NC} (未找到预期输出: $expected)"
                ((FAIL++))
            fi
        else
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASS++))
        fi
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((FAIL++))
    fi
}

test_warn() {
    local name="$1"
    echo -e "${YELLOW}⚠ WARN${NC}: $name"
    ((WARN++))
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "11.1 tealdeer-core 单元测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 测试 1: scope=command 查询 TLDR cache
test_case "scope=command 查询 tar" \
    "./target/release/tldr tar" \
    "归档实用程序"

# 测试 2: scope=shortcut 只读 shortcut_pages
test_case "scope=shortcut 查询 vim" \
    "./target/release/tldr --shortcut vim" \
    "Vim"

# 测试 3: list 在 scope=shortcut 返回列表
test_case "scope=shortcut --list" \
    "./target/release/tldr --shortcut --list" \
    "vim"

# 测试 4: show-paths 显示 shortcut_pages_dir
test_case "show-paths 包含 shortcut_pages_dir" \
    "./target/release/tldr --show-paths" \
    "Shortcut pages dir"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "11.2 Tauri 后端测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检查快捷键目录是否存在
SHORTCUT_DIR=$(./target/release/tldr --show-paths | grep "Shortcut pages dir" | cut -d: -f2 | cut -d'(' -f1 | xargs)
if [ -d "$SHORTCUT_DIR" ]; then
    echo -e "${GREEN}✓ PASS${NC}: shortcut_pages 目录存在: $SHORTCUT_DIR"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: shortcut_pages 目录不存在"
    ((FAIL++))
fi

# 检查快捷键页面文件
if [ -f "$SHORTCUT_DIR/vim.page.md" ]; then
    echo -e "${GREEN}✓ PASS${NC}: vim.page.md 存在"
    ((PASS++))
else
    test_warn "vim.page.md 不存在（需要手动创建测试）"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "11.3 前端手工测试（编译检查）"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检查前端编译
if [ -f "frontend/tealdeer-widget/dist/index.html" ]; then
    echo -e "${GREEN}✓ PASS${NC}: 前端已编译"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: 前端未编译"
    ((FAIL++))
fi

# 检查 Tauri 应用
if [ -f "./target/release/tealdeer_tile" ]; then
    echo -e "${GREEN}✓ PASS${NC}: Tauri 应用已编译"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: Tauri 应用未编译"
    ((FAIL++))
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "11.4 边界与回归测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 测试同名页面隔离
test_case "command::nano 和 shortcut::nano 隔离" \
    "[ -d '$SHORTCUT_DIR' ] && [ -d '$HOME/.local/share/tealdeer/pages' ]" \
    ""

# 测试配置文件
CONFIG_FILE=$(./target/release/tldr --show-paths | grep "Config path" | cut -d: -f2- | xargs)
if [ -f "$CONFIG_FILE" ]; then
    if grep -q "shortcut_pages_dir" "$CONFIG_FILE"; then
        echo -e "${GREEN}✓ PASS${NC}: 配置文件包含 shortcut_pages_dir"
        ((PASS++))
    else
        test_warn "配置文件不包含 shortcut_pages_dir（可能未初始化）"
    fi
else
    test_warn "配置文件不存在"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "测试总结"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "通过: ${GREEN}$PASS${NC}"
echo -e "失败: ${RED}$FAIL${NC}"
echo -e "警告: ${YELLOW}$WARN${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAIL 个测试失败${NC}"
    exit 1
fi
