# Build CLI (tldr)

All commands run at the workspace root (`tealdeer/`).

## Main flows (release by default)

### 1) Release build

```bash
cd /path/to/tealdeer
cargo build --release -p tldr
strip target/release/tldr  # Linux/macOS
# Binary: ./target/release/tldr
```

### 2) Release install

```bash
cd /path/to/tealdeer
cargo install --path tldr
# Verify: tldr --version
```

## Developer option (debug / CI)

```bash
# Debug build (dev only)
cargo build -p tldr
# Binary: ./target/debug/tldr

# CI build for all workspace crates (release)
cargo build --workspace --release
```

For full details, see `docs/build_cli.md`.
