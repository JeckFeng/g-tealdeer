# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Vue 3 UI code (`App.vue`, `main.ts`) and component logic.
- `src/assets/`: bundled UI assets imported from the app.
- `public/`: static files served as-is by Vite.
- `src-tauri/`: Tauri backend; `src-tauri/src/` holds Rust entry points, `src-tauri/tauri.conf.json` is app configuration, `src-tauri/capabilities/` defines permissions, and `src-tauri/icons/` stores app icons.
- `src-tauri/gen/` and `src-tauri/target/`: generated artifacts; avoid manual edits.

## Build, Test, and Development Commands
- `npm install`: install Node dependencies.
- `npm run dev`: start the Vite dev server for the web UI.
- `npm run build`: type-check (`vue-tsc --noEmit`) and build production assets.
- `npm run preview`: serve built assets locally.
- `npm run tauri -- dev`: run the desktop app (requires Rust toolchain).
- `npm run tauri -- build`: build the desktop bundle.

## Coding Style & Naming Conventions
- Vue SFCs use `<script setup lang="ts">`; keep component files in PascalCase (e.g., `MyWidget.vue`).
- Use 2-space indentation in Vue/TS/CSS to match existing files and double quotes in TS imports.
- Use `camelCase` for JS/TS variables and functions; Rust uses `snake_case` per standard conventions.
- No repo-wide lint/formatter config is present; follow existing formatting, and use `cargo fmt` for Rust changes when available.

## Testing Guidelines
- No test runner or `npm test` script is configured yet; add one if you introduce tests.
- Place UI tests near `src/` and Rust tests under `src-tauri/src/`.
- Coverage targets are not defined; report any manual testing in PRs.

## Commit & Pull Request Guidelines
- Commit messages are short, imperative, and capitalized (e.g., `Add search.languages setting`, `Bump actions/checkout from 5 to 6`); include a PR number when applicable.
- PRs should include: a concise summary, linked issues (if any), testing commands run (or "not run"), and screenshots/GIFs for UI changes.
- Note the platforms verified for Tauri changes (macOS, Windows, Linux).

## Security & Configuration Tips
- Keep Tauri permissions minimal; update `src-tauri/capabilities/default.json` when adding APIs or plugins.
- App/window settings live in `src-tauri/tauri.conf.json`; avoid editing generated content in `src-tauri/gen/`.
