# Changelog

All notable changes to **Oryn** are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
