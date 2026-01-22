#!/bin/bash
# Tealdeer CLI 安装脚本（自动配置PATH）

set -e

echo "🚀 Installing tealdeer CLI..."

# 安装CLI
cargo install --path tldr

echo "✅ CLI installed to ~/.cargo/bin/tldr"

# 检测并配置shell
configure_shell() {
    local shell_name=$1
    local config_file=$2
    local path_line='export PATH="$HOME/.cargo/bin:$PATH"'
    
    if [ -f "$config_file" ]; then
        if ! grep -q "cargo/bin" "$config_file" 2>/dev/null; then
            echo "" >> "$config_file"
            echo "# Added by tealdeer installer" >> "$config_file"
            echo "$path_line" >> "$config_file"
            echo "✅ Added PATH to $config_file"
            return 0
        else
            echo "ℹ️  PATH already configured in $config_file"
            return 1
        fi
    fi
    return 1
}

configure_fish() {
    local config_file="$HOME/.config/fish/config.fish"
    local fish_path_line='set -gx PATH $HOME/.cargo/bin $PATH'
    
    # 创建fish配置目录（如果不存在）
    mkdir -p "$HOME/.config/fish"
    
    if [ ! -f "$config_file" ]; then
        touch "$config_file"
    fi
    
    if ! grep -q "cargo/bin" "$config_file" 2>/dev/null; then
        echo "" >> "$config_file"
        echo "# Added by tealdeer installer" >> "$config_file"
        echo "$fish_path_line" >> "$config_file"
        echo "✅ Added PATH to $config_file"
        return 0
    else
        echo "ℹ️  PATH already configured in $config_file"
        return 1
    fi
}

# 检测当前shell
CURRENT_SHELL=$(basename "$SHELL")
CONFIGURED=false

case "$CURRENT_SHELL" in
    fish)
        configure_fish && CONFIGURED=true
        ;;
    zsh)
        configure_shell "zsh" "$HOME/.zshrc" && CONFIGURED=true
        ;;
    bash)
        configure_shell "bash" "$HOME/.bashrc" && CONFIGURED=true
        ;;
    *)
        echo "⚠️  Unknown shell: $CURRENT_SHELL"
        ;;
esac

# 同时配置其他常见shell（如果配置文件存在）
[ "$CURRENT_SHELL" != "bash" ] && configure_shell "bash" "$HOME/.bashrc" 2>/dev/null || true
[ "$CURRENT_SHELL" != "zsh" ] && configure_shell "zsh" "$HOME/.zshrc" 2>/dev/null || true
[ "$CURRENT_SHELL" != "fish" ] && configure_fish 2>/dev/null || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Installation complete!"
echo ""

if [ "$CONFIGURED" = true ]; then
    echo "📝 Next steps:"
    case "$CURRENT_SHELL" in
        fish)
            echo "   source ~/.config/fish/config.fish"
            ;;
        zsh)
            echo "   source ~/.zshrc"
            ;;
        bash)
            echo "   source ~/.bashrc"
            ;;
    esac
    echo ""
    echo "   Or restart your terminal"
else
    echo "⚠️  Please manually add to your shell config:"
    echo ""
    if [ "$CURRENT_SHELL" = "fish" ]; then
        echo "   echo 'set -gx PATH \$HOME/.cargo/bin \$PATH' >> ~/.config/fish/config.fish"
    else
        echo "   echo 'export PATH=\"\$HOME/.cargo/bin:\$PATH\"' >> ~/.$CURRENT_SHELL"rc
    fi
fi

echo ""
echo "🔍 Verify installation:"
echo "   tldr --version"
echo "   tldr pacman"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
