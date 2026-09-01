# Oryn Features Overview

A comprehensive guide to all core capabilities, tools, and workflows in **Oryn**.

---

## 🗂 Dual-Pane & Multi-View Workspace

### Independent Dual Panes
- Two side-by-side file panes (Left and Right) with independent active paths, selections, sort orders, and history.
- Quickly toggle active focus with `Tab` or transfer files between panes with `F5` (Copy) and `F6` (Move).

### Multiple View Modes
- **📋 List View (`⌘1` / `Ctrl+1`):** High-density file list with 60fps virtualized scrolling for large directories with hundreds of thousands of files.
- **🖼 Grid / Icons View (`⌘2` / `Ctrl+2`):** Visual card grid tailored for media files, photo albums, and visual assets.
- **🗃 Miller Columns View (`⌘3` / `Ctrl+3`):** macOS Finder style cascading column navigation with instant folder hierarchy expansion and inspector preview.

### Multi-Tab Management
- Independent tabs in each pane (`⌘T` / `Ctrl+T`, `⌘W` / `Ctrl+W`, `Ctrl+Tab` / `Ctrl+Shift+Tab`).
- Retains distinct navigation history, scroll position, and active selection per tab.

### Live Real-Time Filesystem Watching
- Native Rust watcher (`notify`) automatically detects and updates open panes when external apps create, edit, or delete files.

### Interactive Address Bar & Breadcrumbs
- Clickable breadcrumbs for instant folder navigation.
- Single-click the address bar background or press `⌘L` / `Ctrl+L` to type or paste paths directly.

---

## ⚡ Blazing Fast Transfers & Operations

### High-Throughput Transfer Engine
- Native Rust chunked buffer pipeline with live progress, ETA, and speed counters.
- Non-blocking asynchronous I/O that keeps the UI responsive at 60fps even during multi-gigabyte transfers.

### Interactive Conflict Resolver
- Handles file collisions with Overwrite, Skip, Rename, or Auto-Rename with batch decisions (*"Apply to all"*).

### Background Job Queue
- Run multiple copy/move operations simultaneously without locking the UI.

### Archive Extraction & Creation
- Instant `.zip` and `.tar.gz` packing and multi-threaded extraction with progress feedback.

---

## 👁 Universal Quick Look & Preview

### Rich Syntax Highlighting & Code Preview
- Integrated code preview with line numbers and keyword coloring for over 50 programming languages.

### In-Place File Editor
- Press **Edit** (or `Ctrl+E` / `⌘E`) inside Preview mode to modify text, Markdown, JSON, YAML, or code files with `Tab` indentation and save directly via `Ctrl+S` / `⌘S`.

### Markdown Viewer
- Switch seamlessly between rendered formatted prose and raw markdown source.

### Media Previews
- High-resolution image inspector (dimensions, format, metadata), audio playback, and HTML5 video viewer.

### Office Document Reader
- Instant reader for modern `.docx`, `.xlsx`, and `.pptx` documents without opening external office suites.

---

## 🔍 Fast Search, VFS & Remote Servers

### Rust-Powered Search (`⌘F` / `Ctrl+F`)
- Multi-threaded name and text search with regex, case sensitivity, file size filters, and date constraints.
- Paged virtual search results for immediate responsiveness even across millions of files.

### Virtual File System (VFS)
- Browse inside `.zip` and `.tar.gz` archives seamlessly like regular directories without prior extraction.

### Find & Replace in Files (`⌘⇧H` / `Ctrl+Shift+H`)
- Batch find and replace across file contents with preview diffs before applying changes.

### Remote SFTP / SSH Servers
- Connect to remote Linux and cloud servers, browse remote directories, and transfer files across local and remote panes.

---

## 🛠 Built-in Power Tools

### Multi-Rename Tool (`⌘M` / `Ctrl+M`)
- Batch rename with dynamic pattern tokens (`[N]`, `[E]`, `[C]`, `[YMD]`), regex search/replace, and case converters.

### Disk Space Analyzer
- Visual treemap and size breakdown to identify large files and folders taking up disk space.

### Duplicate File Finder
- Scan directories for identical files using fast multi-threaded SHA-256 cryptographic hashing.

### Integrated Git Panel (`⌘G` / `Ctrl+G`)
- Track Git status, inspect branch diffs and blame, stage/unstage files, and review commit logs.

### Integrated Terminal Drawer (`F9` / `Ctrl+\``)
- Quick terminal accessible from the active directory.

### Command Palette (`⌘K` / `Ctrl+K` / `⌘P`)
- Spotlight-style command runner for every action, tool, and setting in Oryn.
