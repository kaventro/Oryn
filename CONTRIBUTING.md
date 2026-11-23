# Contributing to Oryn

Thanks for your interest in contributing! This guide will help you get started.

## Quick Start

```bash
git clone https://github.com/kaventro/Oryn.git
cd Oryn
cargo run -p oryn-app
```

See [README.md](README.md#run-from-source) for the Linux system libraries.

## Development Workflow

1. **Fork** the repository and create a feature branch from `main`.
2. Make your changes — keep commits focused and descriptive.
3. Run the quality checks locally before pushing:

   ```bash
   cargo fmt -p oryn-app -p oryn-core --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

4. Open a **Pull Request** against `main`.

## Code Style & Architecture

### Rust

- Run `cargo fmt -p oryn-app -p oryn-core` before committing.
- All Clippy warnings are treated as errors in CI (`-D warnings`).
- Prefer `?` and error values over `.unwrap()`, because a panic closes the app.
- Layering:
  - `crates/oryn-core` holds file services, the VFS, safe file operations and the logic behind search, Git, compare, find and replace, checksums, tags, previews and updates. Put testable logic here.
  - `crates/oryn-app` holds the GPUI interface: the workspace, panes, dialogs, Quick View, the terminal and the tool sheets.
- Run slow work off the UI thread with `cx.background_spawn` (see `tools::run_bg`).
- Give every interactive element a role and an accessible label.

## What to Work On

- Check the [Issues](https://github.com/kaventro/Oryn/issues) tab for bugs and feature requests.
- Issues labeled **`good first issue`** are great starting points.
- If you want to tackle something larger, open an issue first to discuss the approach.

## Reporting Bugs

Please use the [Bug Report](https://github.com/kaventro/Oryn/issues/new?template=bug_report.yml) template and include:

- Steps to reproduce
- Expected vs. actual behavior
- OS and Oryn version

## Suggesting Features

Use the [Feature Request](https://github.com/kaventro/Oryn/issues/new?template=feature_request.yml) template.

## License

By contributing, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE).
