# Rust-only UI spike (phase 0)

`crates/oryn-app` is a GPUI Kit build of the Oryn shell with no WebView, HTML, CSS or TypeScript. `crates/oryn-core` holds the file services, the VFS, `fs_safe`, and the logic behind the search, Git, compare, find-and-replace, archive and zip commands, moved out of `desktop/` in phase 1. The Tauri app depends on it through a path dependency, so both apps run the same file code. Tauri-specific parts stay in `desktop/`: the `AppSink` progress sink for transfers, and `TauriEvents`, which implements `oryn_core::events::EventSink` for the folder watcher, folder compare and find-and-replace.

## Run

```sh
cargo run --release -p oryn-app -- [left-dir] [right-dir]
```

| Key | Action |
| --- | --- |
| Up, Down, Page Up, Page Down, Home, End | Move the cursor |
| Left, Right | Move in grid view; step between columns in column view |
| Enter, double-click | Open a folder |
| Backspace, Alt+Up | Parent folder |
| Alt+Left, Alt+Right | Back, forward |
| Space, Insert | Mark or unmark, then move down |
| Ctrl+A (Cmd+A on macOS) | Mark or unmark all |
| Alt+S, Alt+D | Select or deselect by mask, such as `*.png` |
| Alt+I | Invert the selection |
| Alt+A | Clear the selection |
| Alt+E | Select every file with the cursor's extension |
| Ctrl+1, Ctrl+2, Ctrl+3 (Cmd on macOS) | List, grid or column view |
| Ctrl+T, Ctrl+W (Cmd on macOS) | New tab, close tab |
| Ctrl+Tab, Ctrl+Shift+Tab | Next tab, previous tab |
| Tab | Switch pane |
| Ctrl+F (Cmd+F) | Filter the active pane |
| Ctrl+\\ (Cmd+\\) | Dual or single pane |
| Ctrl+B (Cmd+B) | Show or hide the sidebar |
| Ctrl+\` or F9 | Toggle the terminal |
| Ctrl+, (Cmd+,) | Cycle accent trays |
| Ctrl+Q (Cmd+Q) | Quit |

The layout copies the current web UI: the sidebar sections, the 52 px header with the folder tab and icon buttons, the toolbar groups, the breadcrumb and search pills, the file rows and the status bar use the same sizes, colours and SVG icons as `src/`. File icons and colours come from `oryn_core::icons`, a port of `iconRegistry.ts`. The toolbar's file actions, the grid and column views, new tabs and remote servers are drawn but not wired yet.

`ORYN_BENCH=1` scrolls the left pane from top to bottom in 173-row steps and prints frame timings. `ORYN_TRAY=0..5` picks the starting accent.

## Results on Linux (2026-11-22)

Measured on a 4-core cloud VM under Xvfb with Mesa lavapipe (CPU Vulkan), release build.

| Check | Result |
| --- | --- |
| Builds against `gpui-kit = "=0.10.0"` | Yes, with Rust 1.100 |
| 100,050-row folder, listing time | 180 ms (3 runs: 182, 182, 179) |
| Scroll frame time, 100k rows vs 2k rows | 47.9 ms vs 46.7 ms; flat with list size |
| Idle memory (PSS), demo folders, 3 runs | 152 MiB (155,617 to 156,041 kB) |
| Idle memory (PSS), 100k-row folder | 180 to 189 MiB |
| Release binary size | 55 MB, unstripped |
| Terminal (alacritty_terminal + portable-pty) | Shell, colours, cursor, resize |
| Accent trays from DESIGN.md | All six |

Frame times and memory include CPU rasterisation by lavapipe, which a real GPU does not pay. The same software-rendering setup measured Tauri at about 336 MiB PSS in HeyVasily/rust-desktop-comparison.

## Not yet verified

- macOS and Windows builds: `.github/workflows/rust-app.yml` builds and tests all three.
- Wayland, a real GPU, and frame rate on hardware.
- Screen readers. Rows and controls set AccessKit roles and labels, but VoiceOver, NVDA and Orca were not available here.
- IME text entry, because the spike has no text fields yet.
