# Tealdeer Agent Notes

## Project Overview
tealdeer is a Rust implementation of the tldr client. The CLI entry point is the `tldr` binary; the code path is `src/main.rs` -> `src/cli.rs` -> config/cache/output modules. Pages are resolved from a local cache plus optional custom pages/patches.

## Source Layout
- `src/`: core CLI, cache, config, rendering, and formatting logic.
- `tests/`: integration tests, fixtures under `tests/cache/`, `tests/custom-pages/`, `tests/rendered/`.
- `completion/`: shell completion scripts.
- `docs/`: user documentation source.
- `frontend/`: UI assets and the `tealdeer-widget` subproject.
- `scripts/`: helper scripts used in release/testing workflows.

## Runtime Flow (High Level)
1. Parse CLI flags in `src/cli.rs` (clap).
2. Load config via `ConfigLoader` in `src/config.rs` (supports `--config-path` and env overrides).
3. Build a `CacheConfig` in `src/main.rs`, open/update cache in `src/cache.rs`.
4. Resolve a page (`find_page`), optionally apply a patch, and render via `src/output.rs` + `src/formatter.rs`.

## CLI Options -> Code Source Map
All flags are defined in `src/cli.rs` on `struct Cli`. Handling is in `src/main.rs` unless noted.
- `COMMAND...`: `Cli.command` -> normalized to `command.join("-").to_lowercase()` in `src/main.rs` and looked up via `Cache::find_page` (`src/cache.rs`).
- `-l, --list`: `Cli.list` -> `cache.list_pages()` in `src/main.rs`.
- `--edit-page`: `Cli.edit_page` -> `spawn_editor()` in `src/main.rs` writes `<command>.page.md`.
- `--edit-patch`: `Cli.edit_patch` -> `spawn_editor()` in `src/main.rs` writes `<command>.patch.md`.
- `-f, --render FILE`: `Cli.render` -> `PageLookupResult::with_page` + `print_page` in `src/output.rs`.
- `-p, --platform PLATFORM`: `Cli.platforms` -> `config.search.platforms` in `src/main.rs`; enum values in `src/types.rs`.
- `-L, --language LANG`: `Cli.language` -> search/download languages set in `src/main.rs`.
- `-u, --update`: `Cli.update` -> `update_cache()` in `src/main.rs` -> `Cache::update` in `src/cache.rs`.
- `--no-auto-update`: `Cli.no_auto_update` -> gates auto updates in `src/main.rs`.
- `-c, --clear-cache`: `Cli.clear_cache` -> `clear_cache()` in `src/main.rs` -> `Cache::clear` in `src/cache.rs`.
- `--config-path FILE`: `Cli.config_path` -> `ConfigLoader::read` in `src/config.rs`.
- `--pager`: `Cli.pager` -> `configure_pager()` and `print_page()` in `src/output.rs`.
- `-r, --raw`: `Cli.raw` -> raw markdown path in `src/output.rs`.
- `-q, --quiet`: `Cli.quiet` -> suppresses warnings/messages in `src/main.rs`.
- `--show-paths`: `Cli.show_paths` -> `show_paths()` in `src/main.rs`.
- `--seed-config`: `Cli.seed_config` -> `make_default_config()` in `src/config.rs`.
- `--color WHEN`: `Cli.color` -> `ColorOptions` in `src/types.rs`, styling decision in `src/main.rs`.
- `-v, --version`: `Cli.version` -> clap version output in `src/cli.rs`.
- `-h, --help`: clap default; help template configured in `src/cli.rs`.

## Config and Environment
- Config file: `config.toml` (default path in `src/config.rs`).
- Env overrides: `TEALDEER_CONFIG_DIR` (config dir), `TEALDEER_CACHE_DIR` (deprecated), `LANG`/`LANGUAGE` (language fallback), `EDITOR` (custom page editing), `NO_COLOR` (disable styles).
- Custom pages: `<custom_pages_dir>/<command>.page.md` and patches `<command>.patch.md` (see `Cache::find_page` in `src/cache.rs`).

## Tests
- `cargo test` runs unit + integration tests.
- Integration test harness is in `tests/lib.rs` (asserts CLI behavior, cache layout, and output rendering).

## Adding/Changing CLI Behavior
1. Add or update flags in `src/cli.rs`.
2. Wire behavior in `src/main.rs`.
3. Update config defaults or parsing in `src/config.rs` if needed.
4. Add tests in `tests/lib.rs` with fixtures under `tests/`.
