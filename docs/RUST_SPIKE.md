# Rust-only UI spike (phase 0)

`crates/oryn-app` is a GPUI Kit build of the Oryn shell with no WebView, HTML, CSS or TypeScript. `crates/oryn-core` reuses `desktop/src/services/fs_listing.rs` as is, so both apps list folders with the same code.

## Run

```sh
cargo run --release -p oryn-app -- [left-dir] [right-dir]
```

| Key | Action |
| --- | --- |
| Up, Down, Page Up, Page Down, Home, End | Move the cursor |
| Enter, double-click | Open a folder |
| Backspace | Parent folder |
| Space, Insert | Mark or unmark, then move down |
| Ctrl+A (Cmd+A on macOS) | Mark or unmark all |
| Tab | Switch pane |
| Ctrl+` | Toggle the terminal |
| Ctrl+Shift+T (Cmd+Shift+T on macOS) | Cycle accent trays |
| Ctrl+Q (Cmd+Q on macOS) | Quit |

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
