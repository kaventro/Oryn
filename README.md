<p align="center">
  <img src="desktop/icons/128x128@2x.png" width="120" height="120" alt="Oryn Logo" />
</p>

<h1 align="center">Oryn</h1>

<p align="center">
  <strong>The modern, blazing-fast, keyboard-driven dual-pane file manager</strong><br />
  Built with <a href="https://v2.tauri.app">Tauri 2.11</a> (Rust 1.98) and TypeScript 7 / Vite — lightweight, native, and cross-platform.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-AGPL_3.0-blue.svg" alt="License: AGPL-3.0" /></a>
  <a href="https://v2.tauri.app"><img src="https://img.shields.io/badge/Tauri-2.11.x-FFC131?logo=tauri&logoColor=white" alt="Tauri 2.11.x" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.98.0+-orange?logo=rust&logoColor=white" alt="Rust 1.98.0+" /></a>
  <a href="https://www.typescriptlang.org"><img src="https://img.shields.io/badge/TypeScript-7.0+-3178c6?logo=typescript&logoColor=white" alt="TypeScript 7.0+" /></a>
  <img src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-4c1" alt="Platform: macOS | Windows | Linux" />
</p>

---

## ✨ Overview

**Oryn** combines the raw speed, keyboard mastery, and dual-pane efficiency of classic power tools (*Total Commander*, *Far Manager*) with the refined aesthetics of **macOS Sequoia** and **Windows 11 Mica**.

Unlike heavy Electron-based file managers, Oryn boots in **under 300ms**, consumes minimal memory, and leverages high-performance **Rust** engines for directory listing, file transfers, ripgrep content searches, and real-time filesystem monitoring.

---

## 🚀 Features Highlights

- 🗂 **[Dual-Pane & Multi-View Workspace](docs/FEATURES.md#-dual-pane--multi-view-workspace)** — Side-by-side panes, multi-tabs, List / Grid / Miller Columns views, interactive breadcrumb navigation, and live filesystem watching.
- ⚡ **[High-Throughput Transfers](docs/FEATURES.md#-blazing-fast-transfers--operations)** — Non-blocking Rust transfer pipeline, conflict resolution (*Overwrite / Skip / Auto-Rename*), background jobs, and `.zip` / `.tar.gz` archive creation and extraction.
- 👁 **[Universal Preview & In-Place Editor](docs/FEATURES.md#-universal-quick-look--preview)** — Syntax highlighting for 50+ languages, in-place code editing, Markdown reader, media viewer, and Office documents (`.docx`, `.xlsx`, `.pptx`).
- 🔍 **[Search, VFS & Remote Servers](docs/FEATURES.md#-fast-search-vfs--remote-servers)** — Multi-threaded regex file search, transparent archive Virtual File System (VFS), batch find & replace, and SFTP / SSH remote manager.
- 🛠 **[Integrated Power Tools](docs/FEATURES.md#-built-in-power-tools)** — Pattern-based batch rename tool, disk space treemap analyzer, SHA-256 duplicate finder, Git status/diff panel, terminal drawer, and spotlight command palette.

📖 *For a detailed breakdown of all capabilities, see the full **[Features Documentation](docs/FEATURES.md)**.*

---

## ⌨️ Essential Keyboard Shortcuts

| Shortcut (macOS) | Shortcut (Win / Linux) | Action |
| :--- | :--- | :--- |
| `Tab` | `Tab` | Switch focus between Left and Right panes |
| `⌘1` / `⌘2` / `⌘3` | `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Switch View (List / Grid / Miller Columns) |
| `Space` / `F3` | `Space` / `F3` | Quick Look Preview / Compute Folder Size |
| `F5` / `F6` | `F5` / `F6` | Copy / Move selection to opposite pane |
| `F7` / `Shift+F7` | `F7` / `Shift+F7` | Create Folder (`mkdir`) / Create File |
| `F8` / `Delete` | `F8` / `Delete` | Move selection to Trash |
| `⌘P` / `⌘K` | `Ctrl+P` / `Ctrl+K` | Spotlight Command Palette |
| `⌘F` / `Alt+F7` | `Ctrl+F` / `Alt+F7` | Quick Filter Bar / Full Search Dialog |
| `⌘M` | `Ctrl+M` | Batch Multi-Rename Tool |
| `⌘,` | `Ctrl+,` | Preferences & Keymap Customization |

⌨️ *For complete shortcut tables including selection masks, archive actions, Git controls, and VIM mode, see the **[Shortcuts Reference](docs/SHORTCUTS.md)**.*

---

## 🏗 Architecture & Tech Stack

Oryn is built with a lightweight, zero-overhead philosophy:
- **Backend:** [Rust](https://www.rust-lang.org/) (≥ 1.98.0) with [Tauri 2](https://v2.tauri.app/) (2.11.x) — memory-safe native filesystem APIs, Virtual File System (VFS) for archives, background transfer pipeline, `notify` event watcher, and multithreaded search engine.
- **Frontend:** [TypeScript 7](https://www.typescriptlang.org/) + [Vite](https://vite.dev/) + Modular CSS — zero framework overhead, 60fps virtualized scrolling, sub-second HMR, and sub-300ms application startup.
- **Design System:** Apple Human Interface Guidelines aesthetic with frosted glass vibrancy, dark/light themes, and Windows 11 Mica compatibility.

🏛 *For detailed subsystem diagrams, full directory layout, and VFS/IPC details, see the **[Architecture Documentation](docs/ARCHITECTURE.md)**.*

---

## 📦 Getting Started

### Prerequisites
- **Node.js** ≥ 20
- **Rust** ≥ 1.98.0 (`rustup default stable`)
- Platform build dependencies:
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Windows:** Visual Studio C++ Build Tools
  - **Linux:** `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`

### Development Setup

```bash
# 1. Clone the repository
git clone https://github.com/kaventro/Oryn.git
cd Oryn

# 2. Install frontend dependencies
npm install

# 3. Launch in development mode (Vite HMR + Tauri backend)
npm run dev
```

### Running Tests & Quality Checks

```bash
# Typecheck TypeScript (TypeScript 7)
npm run typecheck

# Run frontend TypeScript unit test suite (89+ tests)
npm run ui:test

# Run Rust backend unit test suite (86+ tests)
cargo test --manifest-path desktop/Cargo.toml

# Check production frontend build
npm run ui:build
```

### Production Build

```bash
# Build native standalone executables and platform installers (.dmg / .exe / .AppImage / .deb)
npm run build
```

---

## 📄 License

Oryn is open-source software licensed under the [GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE).
Feel free to use it, contribute, and build upon it.
