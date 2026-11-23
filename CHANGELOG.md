# Changelog

All notable changes to **Oryn** are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.0.10] - 2026-11-23

### Fixed
- Oryn for macOS no longer quits at launch with "Library not loaded: libssl". OpenSSL is now built into the app.
- The macOS app and `.dmg` are signed with a Developer ID and notarized, so Gatekeeper opens them without a warning.

### Security
- Every action in the release workflow is pinned to a commit SHA.

## [0.0.9] - 2026-11-23

### Added
- Add any folder to Favourites with the + next to FAVORITES, the context menu, the More menu or Ctrl+D. Remove one with the × that appears on hover. Favourites are saved.
- Saved SFTP servers are listed in the sidebar and connect from there.
- Enter and double-click open files in their default app. A `.zip` or `.tar` archive opens like a folder; files inside can be previewed, opened and copied out.
- Git status letters and tag colours next to each file in the list.
- The More menu (…) in the toolbar: view, sort, hidden files, new file and folder, favourites, terminal, properties, disk space, duplicates, swap panes and go to folder.
- Properties (Alt+Enter or Ctrl+I) with location, kind, size, date and permissions. Folder sizes are counted in the background.
- Cut, Copy and Paste (Ctrl+X, Ctrl+C, Ctrl+V), Swap Panes (Ctrl+U), Open in Terminal, Show Hidden Files (Ctrl+H, Cmd+Shift+. on macOS) and Go to Folder (Ctrl+L, or the pencil in the path bar).
- Sidebar sections collapse when you click their titles, and stay that way.
- Oryn reopens the folders you had open last time.
- UI tests that click through favourites, opening files, archives, the toolbar, the sidebar and the new commands.

### Changed
- Names sort naturally, so file2 comes before file10.
- The search box shows a clear button instead of an unused filter icon.
- Clicking a sidebar tag tags the marked files, then lists everything with that tag.

### Fixed
- The + buttons, section arrows, More button and path pencil did nothing.
- A failed Git commit no longer clears the commit message.

## [0.0.8] - 2026-11-23

Oryn is now a native Rust app. The Tauri, TypeScript and Vite app is gone, and so is the WebView: the whole interface is drawn on the GPU with GPUI Kit, using the same layout, colours and icons as before.

### Added
- Dual panes with tabs, list, grid (with image thumbnails) and column views, a sidebar with favourites, drives, SFTP servers and colour tags, and a command palette that also jumps to typed paths.
- Copy, move, duplicate, rename, batch rename, new file and folder, Trash and permanent delete, drag and drop, and a conflict prompt before anything is overwritten. Long operations run in the background with progress and Cancel.
- Compress to ZIP and Extract Here for `.zip`, `.tar` and `.tar.gz` archives.
- Folders refresh on their own when files change on disk.
- Quick View for code (highlighted), Markdown, images, text, office documents, audio, video and PDF, plus a hex view.
- Power tools: search, find and replace, Git (stage, commit, diff, branches, pull, push), folder compare and sync, duplicates, checksums, disk space, tags and SFTP.
- A terminal drawer that follows the active pane.
- Settings for accent colour, view, hidden files, delete confirmation, Trash, dual pane, sidebar and update checks.
- An update check on start that shows an Update button in the header when a newer release is out.

### Changed
- Installers: a `.dmg` for Apple silicon Macs, an NSIS installer and a portable `.exe` for Windows, and a `.deb` and `.tar.gz` for Linux. Each platform's SHA-256 sums are attached to the release.
- Settings and tags are stored in the Oryn data folder. Settings from 0.0.7 and earlier are not carried over.
- Apps from 0.0.7 and earlier do not update themselves to this release. Install it from the release page.

### Removed
- The Tauri backend, the TypeScript and Vite frontend, Node.js tooling and the Playwright suite.

---

## [0.0.6] - 2026-09-24

### 🚀 Performance & Backend
- **APFS Clonefile Support:** Integrated instant hardware Copy-on-Write zero-byte file cloning on macOS APFS (`libc::clonefile`), speeding up transfers up to 6.1x.
- **Cross-Platform Buffered Fallback:** Extracted `copy_file_buffered` for Windows, Linux, and non-APFS partitions with full integrity checks and 256 KB streaming buffers.
- **IPC Event Throttling:** Throttled file transfer progress events to 60ms intervals (max ~16 events/sec), eliminating webview message queue saturation and UI freezes during large transfers.
- **Directory Sorting Optimization:** Replaced quadratic string comparisons in `fs_listing` with `sort_by_cached_key`, cutting ~120,000 heap allocations and improving 10k-file sort latency by 1.54x.
- **Miller Columns Batch DOM Rendering:** Replaced incremental `appendChild` DOM manipulation with atomic `container.replaceChildren(...builtNodes)` to eliminate layout thrashing.

### ✨ Features & Improvements
- **Grid View & Lazy Thumbnails:** Introduced `ThumbnailCache` with asynchronous lazy loading, in-memory LRU caching, and smooth fallback to SVG icons for media files in Grid and List modes.
- **SFTP / Remote Connections:** Enabled SFTP by default (`enableSftp: true`) with full UI integration in the sidebar, header toolbar, and polished modal styling for SSH/SFTP profile management.
- **Enhanced Terminal Drawer:** Added ANSI color parsing (`ansiToHtml`) for CLI tools, real-time working directory synchronization with the active file pane, and command history shortcuts.
- **Automated E2E Playwright Suite:** Built headless Chromium E2E test runner (`scripts/e2e_playwright_runner.py`) testing Dual Pane, Grid View, Miller Columns, SFTP Dialog, Terminal Drawer, and Preferences with verified screenshots.

### 🤖 CI / CD & DevOps
- **GitHub Actions Playwright CI:** Integrated automated Playwright Chromium E2E testing into `.github/workflows/ci.yml` on Ubuntu runners with screenshot artifact uploading.
- **Privacy & Hygiene:** Purged all local personal paths and internal state from repository test scripts and test fixtures.
