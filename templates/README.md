# Shortcut Page Templates

This directory contains templates and examples for creating shortcut pages.

## Template

Use `shortcut_page_template.md` as a starting point for creating new shortcut pages.

## Examples

- `vim_shortcuts_example.md` - Vim editor shortcuts
- `vscode_shortcuts_example.md` - Visual Studio Code shortcuts

## Usage

### CLI

1. Copy a template or example:
   ```bash
   cp templates/shortcut_page_template.md ~/.local/share/tealdeer/shortcut_pages/myapp.page.md
   ```

2. Edit the file with your shortcuts

3. View the shortcuts:
   ```bash
   tldr --shortcut myapp
   ```

CLI shortcut pages directory: `~/.local/share/tealdeer/shortcut_pages/`  
Desktop app shortcut pages directory: `~/.local/share/com.xian00.tealdeer-tile/shortcut_pages/`

### GUI

1. Open Tealdeer-Tile application
2. Go to "New Page" tab
3. Select "Shortcut" type
4. Enter application name and shortcuts
5. Click "Create"

## Format Guidelines

- Use `#` for application name (first line)
- Use `>` for description (second line)
- Use `-` for action descriptions
- Use backticks for key combinations: `` `Ctrl + Key` ``
- Leave blank lines between sections

## Key Notation

- Single key: `` `A` ``, `` `F1` ``
- Modifier + key: `` `Ctrl + A` ``
- Multiple modifiers: `` `Ctrl + Shift + A` ``
- Special keys: `` `Enter` ``, `` `Tab` ``, `` `Esc` ``
- Special characters: `` `Ctrl + [` ``, `` `Ctrl + ]` ``

## Tips

- Keep descriptions concise and clear
- Group related shortcuts together
- Use consistent key notation
- Test shortcuts in the application before documenting
