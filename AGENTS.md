# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust CLI implementation (`src/main.rs` entry, supporting modules in `src/*.rs`).
- `tests/`: integration tests in `tests/lib.rs`; fixtures in `tests/cache` and `tests/custom-pages`, expected output in `tests/rendered`.
- `docs/`: mdBook sources in `docs/src/` with screenshots and assets; `docs/book.toml` config.
- `completion/`: shell completion scripts for bash, fish, and zsh.
- `frontend/tealdeer-widget/`: optional Tauri + Vue desktop UI (follow `frontend/tealdeer-widget/AGENTS.md` when editing).
- `benchmarks/` and `scripts/`: benchmarking Dockerfile and release helper scripts.

## Build, Test, and Development Commands
- `cargo build`: debug CLI build.
- `cargo build --release`: optimized release build.
- `cargo build --features logging`: enable logging; pair with `RUST_LOG=tldr=debug`.
- `cargo test`: run integration tests in `tests/`.
- `cargo test --features ignore-online-tests`: skip network-dependent update tests.
- `rustup component add clippy` then `cargo clippy`: lint checks.
- `cargo fmt`: format Rust using default rustfmt settings (`rustfmt.toml`).
- `cd docs && mdbook build`: build documentation locally.
- `cd docs && mdbook serve`: serve docs on `localhost:3000`.
- `cd frontend/tealdeer-widget && npm install && npm run dev`: run the widget UI.
- `cd frontend/tealdeer-widget && npm run tauri -- dev`: run the desktop app.

## Coding Style & Naming Conventions
- Rust follows rustfmt defaults and standard naming (`snake_case` functions/modules, `UpperCamelCase` types).
- Keep modules small and focused under `src/`; prefer adding helpers to existing domain files (for example `src/config.rs`).
- Frontend styling and naming rules live in `frontend/tealdeer-widget/AGENTS.md`.

## Testing Guidelines
- Add or update fixtures in `tests/cache` and expected output in `tests/rendered` when changing rendering or formatting.
- Use `tests/custom-pages` for page/patch behavior coverage.
- If your change touches network updates, run the online tests or note why they were skipped.

## Commit & Pull Request Guidelines
- Current commit history uses short, descriptive subjects (often in Chinese) with no prefixes or issue IDs; keep messages concise and descriptive.
- No PR template is present; include a summary, rationale, and tests run (or "not run").
- For widget UI changes, add screenshots or GIFs and follow `frontend/tealdeer-widget/AGENTS.md`.
