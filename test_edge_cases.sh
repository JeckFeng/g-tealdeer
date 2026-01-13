#!/bin/bash

echo "═══════════════════════════════════════════════════════════════"
echo "  边界与回归测试"
echo "═══════════════════════════════════════════════════════════════"
echo ""

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0

SHORTCUT_DIR=$(./target/release/tldr --show-paths | grep "Shortcut pages dir" | cut -d: -f2 | cut -d'(' -f1 | xargs)

echo "测试 1: 特殊字符快捷键"
cat > "$SHORTCUT_DIR/test-special.page.md" << 'SPECIAL'
# test-special

> 特殊字符测试

- 方括号:

`Ctrl + [`

- 多修饰键:

`Alt + Shift + F12`
SPECIAL

if ./target/release/tldr --shortcut test-special | grep -q "Ctrl + \["; then
    echo -e "${GREEN}✓ PASS${NC}: 特殊字符渲染正常"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: 特殊字符渲染失败"
    ((FAIL++))
fi

echo ""
echo "测试 2: 同名页面隔离"

# 创建 command::nano
PAGES_DIR="$HOME/.local/share/tealdeer/pages"
mkdir -p "$PAGES_DIR"
cat > "$PAGES_DIR/nano.page.md" << 'NANO_CMD'
# nano

> 命令版本

- 打开文件:

`nano file.txt`
NANO_CMD

# 创建 shortcut::nano
cat > "$SHORTCUT_DIR/nano.page.md" << 'NANO_KEY'
# nano

> 快捷键版本

- 保存:

`Ctrl + O`
NANO_KEY

# 测试隔离
CMD_OUTPUT=$(./target/release/tldr nano 2>&1)
KEY_OUTPUT=$(./target/release/tldr --shortcut nano 2>&1)

if echo "$CMD_OUTPUT" | grep -q "命令版本" && echo "$KEY_OUTPUT" | grep -q "快捷键版本"; then
    echo -e "${GREEN}✓ PASS${NC}: 同名页面完全隔离"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: 同名页面隔离失败"
    ((FAIL++))
fi

echo ""
echo "测试 3: 向后兼容 - 现有功能不受影响"

# 测试普通命令
if ./target/release/tldr tar | grep -q "归档实用程序"; then
    echo -e "${GREEN}✓ PASS${NC}: 普通命令查询正常"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: 普通命令查询失败"
    ((FAIL++))
fi

# 测试 list 命令
LIST_COUNT=$(./target/release/tldr --list | wc -l)
if [ "$LIST_COUNT" -gt 1000 ]; then
    echo -e "${GREEN}✓ PASS${NC}: list 命令正常 ($LIST_COUNT 项)"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: list 命令异常"
    ((FAIL++))
fi

echo ""
echo "测试 4: 禁用/启用功能"

# 禁用快捷键页面
mv "$SHORTCUT_DIR/test-special.page.md" "$SHORTCUT_DIR/test-special.page.md.disabled"

if ./target/release/tldr --shortcut --list | grep -q "test-special"; then
    echo -e "${RED}✗ FAIL${NC}: 禁用的页面仍在列表中"
    ((FAIL++))
else
    echo -e "${GREEN}✓ PASS${NC}: 禁用功能正常"
    ((PASS++))
fi

# 启用回来
mv "$SHORTCUT_DIR/test-special.page.md.disabled" "$SHORTCUT_DIR/test-special.page.md"

if ./target/release/tldr --shortcut --list | grep -q "test-special"; then
    echo -e "${GREEN}✓ PASS${NC}: 启用功能正常"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}: 启用功能失败"
    ((FAIL++))
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "测试总结"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "通过: ${GREEN}$PASS${NC}"
echo -e "失败: ${RED}$FAIL${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ 所有边界测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAIL 个测试失败${NC}"
    exit 1
fi
