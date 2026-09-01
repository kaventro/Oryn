# Contributing to Oryn

Thanks for your interest in contributing! This guide will help you get started.

## Quick Start

```bash
git clone https://github.com/kaventro/Oryn.git
cd Oryn
npm install
npm run dev
```

See [README.md](README.md#getting-started) for full prerequisites (Rust, Node.js, system libraries).

## Development Workflow

1. **Fork** the repository and create a feature branch from `main`.
2. Make your changes — keep commits focused and descriptive.
3. Run the quality checks locally before pushing:

   ```bash
   # Rust
   cd desktop
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test

   # Frontend & TypeScript
   cd ..
   npm run typecheck
   npm run ui:test
   npm run ui:build
   ```

4. Open a **Pull Request** against `main`.

## Code Style & Architecture

### Rust

- Run `cargo fmt` before committing.
- All Clippy warnings are treated as errors in CI (`-D warnings`).
- Prefer `map_err` / `?` over `.unwrap()` in command handlers — panics crash the app.
- Layering:
  - `desktop/src/commands/` — Tauri invoke commands (IPC handlers, response serialization).
  - `desktop/src/services/` — Core business logic (file transfers, duplicate finder, watchers, archives).
  - `desktop/src/vfs/` — Virtual File System providers (local filesystem, archives like `.zip` and `.tar`).

### TypeScript / Frontend

- Vanilla TypeScript (TypeScript 7 + Vite) — zero UI framework overhead.
- Controllers live in `src/modules/`; shared utilities in `src/modules/formatUtils.ts`.
- Always use `escHtml()` when interpolating user-controlled strings into `innerHTML`.
- Write unit tests for new controller and utility logic (`*.test.ts`).

### CSS

- Styles are modularized in `src/styles/**/*.css` and imported via `src/styles.css`.
- Use CSS custom properties (`var(--…)`) instead of hardcoded colors.

## What to Work On

- Check the [Issues](https://github.com/kaventro/Oryn/issues) tab for bugs and feature requests.
- Issues labeled **`good first issue`** are great starting points.
- If you want to tackle something larger, open an issue first to discuss the approach.

## Reporting Bugs

Please use the [Bug Report](https://github.com/kaventro/Oryn/issues/new?template=bug_report.yml) template and include:

- Steps to reproduce
- Expected vs. actual behavior
- OS, Rust version, Node version

## Suggesting Features

Use the [Feature Request](https://github.com/kaventro/Oryn/issues/new?template=feature_request.yml) template.

## License

By contributing, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE).
