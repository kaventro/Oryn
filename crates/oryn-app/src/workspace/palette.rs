use std::path::PathBuf;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::ui::arrange::fuzzy_score;

use super::Workspace;
use crate::actions::*;
use crate::icons::icon;
use crate::theme::{Palette, black_alpha, hex, white_alpha};

type Make = fn() -> Box<dyn Action>;

pub struct CommandPalette {
    input: Entity<InputState>,
    selected: usize,
    focus: FocusHandle,
}

pub const COMMANDS: &[(&str, &str, &str, Make)] = &[
    ("folder-open", "Open", "Enter", || Box::new(OpenEntry)),
    ("copy", "Copy to Other Pane", "F5", || Box::new(CopyToOther)),
    ("folder-input", "Move to Other Pane", "F6", || {
        Box::new(MoveToOther)
    }),
    ("copy-plus", "Duplicate", "Shift+F5", || {
        Box::new(DuplicateItems)
    }),
    ("pencil", "Rename", "F2", || Box::new(RenameItem)),
    ("text-cursor-input", "Batch Rename", "Ctrl+M", || {
        Box::new(MultiRename)
    }),
    ("folder-plus", "New Folder", "F7", || Box::new(NewFolder)),
    ("file-plus", "New File", "Shift+F7", || Box::new(NewFile)),
    ("trash", "Move to Trash", "F8", || Box::new(DeleteItems)),
    ("archive-x", "Delete Permanently", "Shift+Del", || {
        Box::new(DeletePermanently)
    }),
    ("file-archive", "Compress to ZIP", "", || {
        Box::new(CompressZip)
    }),
    ("package-open", "Extract Here", "", || Box::new(ExtractHere)),
    ("clipboard", "Copy Path", "", || Box::new(CopyPath)),
    ("refresh-cw", "Refresh", "Ctrl+R", || Box::new(RefreshPanes)),
    ("list-checks", "Select by Mask", "Alt+S", || {
        Box::new(SelectByMask)
    }),
    ("list-checks", "Deselect by Mask", "Alt+D", || {
        Box::new(DeselectByMask)
    }),
    ("list-checks", "Invert Selection", "Alt+I", || {
        Box::new(InvertSelection)
    }),
    ("list-checks", "Deselect All", "Alt+A", || {
        Box::new(ClearSelection)
    }),
    ("list-checks", "Select by Extension", "Alt+E", || {
        Box::new(SelectByExtension)
    }),
    ("files", "List View", "Ctrl+1", || Box::new(ViewList)),
    ("files", "Grid View", "Ctrl+2", || Box::new(ViewGrid)),
    ("files", "Column View", "Ctrl+3", || Box::new(ViewColumns)),
    ("folder-plus", "New Tab", "Ctrl+T", || Box::new(NewTab)),
    ("x", "Close Tab", "Ctrl+W", || Box::new(CloseTab)),
    ("files", "Dual or Single Pane", "Ctrl+\\", || {
        Box::new(TogglePanes)
    }),
    ("files", "Show or Hide Sidebar", "Ctrl+B", || {
        Box::new(ToggleSidebar)
    }),
    ("terminal", "Toggle Terminal", "Ctrl+`", || {
        Box::new(ToggleTerminal)
    }),
    ("search", "Filter Files", "Ctrl+F", || Box::new(FocusSearch)),
    ("search", "Search Files", "Ctrl+Shift+F", || {
        Box::new(ShowSearch)
    }),
    (
        "replace",
        "Find and Replace in Files",
        "Ctrl+Shift+H",
        || Box::new(ShowReplace),
    ),
    ("git-branch", "Git", "Ctrl+Shift+G", || Box::new(ShowGit)),
    (
        "git-compare",
        "Compare and Sync Folders",
        "Ctrl+Shift+D",
        || Box::new(ShowCompare),
    ),
    ("files", "Find Duplicates", "Ctrl+Shift+U", || {
        Box::new(ShowDuplicates)
    }),
    ("hash", "Checksums", "Ctrl+Shift+K", || {
        Box::new(ShowChecksums)
    }),
    ("chart-pie", "Disk Space", "Ctrl+Shift+S", || {
        Box::new(ShowDiskSpace)
    }),
    ("tag", "Tags", "", || Box::new(ShowTags)),
    ("server", "SFTP", "Ctrl+Shift+R", || Box::new(ShowSftp)),
    ("settings", "Settings", "Ctrl+,", || Box::new(OpenSettings)),
    ("palette", "Next Accent Colour", "", || Box::new(CycleTray)),
    ("star", "Add to Favourites", "Ctrl+D", || {
        Box::new(AddFavorite)
    }),
    ("eye-off", "Show or Hide Hidden Files", "Ctrl+H", || {
        Box::new(ToggleHidden)
    }),
    ("arrow-up-down", "Sort by Name", "", || Box::new(SortByName)),
    ("arrow-up-down", "Sort by Date Modified", "", || {
        Box::new(SortByDate)
    }),
    ("arrow-up-down", "Sort by Size", "", || Box::new(SortBySize)),
    ("info", "Properties", "Alt+Enter", || {
        Box::new(ShowProperties)
    }),
    ("square-terminal", "Open in Terminal", "", || {
        Box::new(OpenInTerminal)
    }),
    ("arrow-left-right", "Swap Panes", "Ctrl+U", || {
        Box::new(SwapPanes)
    }),
    ("scissors", "Cut", "Ctrl+X", || Box::new(CutItems)),
    ("copy", "Copy", "Ctrl+C", || Box::new(CopyItems)),
    ("clipboard-paste", "Paste", "Ctrl+V", || {
        Box::new(PasteItems)
    }),
    ("folder-open", "Go to Folder", "Ctrl+L", || {
        Box::new(GoToPath)
    }),
];

pub(super) fn expand(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix('~') {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        return Some(PathBuf::from(home).join(rest.trim_start_matches(['/', '\\'])));
    }
    let p = PathBuf::from(raw);
    p.is_absolute().then_some(p)
}

impl Workspace {
    pub(super) fn open_palette(
        &mut self,
        _: &OpenPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Type a command, or a path to jump to")
        });
        cx.subscribe_in(
            &input,
            window,
            |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => {
                    if let Some(p) = &mut this.palette {
                        p.selected = 0;
                    }
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.run_palette(window, cx),
                _ => {}
            },
        )
        .detach();
        let first = input.clone();
        cx.defer_in(window, move |_, window, cx| {
            first.update(cx, |s, cx| s.focus(window, cx));
        });
        self.palette = Some(CommandPalette {
            input,
            selected: 0,
            focus: cx.focus_handle(),
        });
        cx.notify();
    }

    fn matches(&self, cx: &App) -> Vec<usize> {
        let Some(p) = &self.palette else {
            return Vec::new();
        };
        let query = p.input.read(cx).value().to_string();
        let mut hits: Vec<(i64, usize)> = COMMANDS
            .iter()
            .enumerate()
            .map(|(i, c)| (fuzzy_score(c.1, &query), i))
            .filter(|(s, _)| query.is_empty() || *s > 0)
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, i)| i).collect()
    }

    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = None;
        self.focus_pane(self.active, window, cx);
    }

    fn run_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = &self.palette else {
            return;
        };
        let query = p.input.read(cx).value().to_string();
        if let Some(path) = expand(&query) {
            if path.is_dir() {
                self.close_palette(window, cx);
                self.go_to(path, window, cx);
            }
            return;
        }
        let selected = p.selected;
        let Some(ix) = self.matches(cx).get(selected).copied() else {
            return;
        };
        self.close_palette(window, cx);
        window.dispatch_action((COMMANDS[ix].3)(), cx);
    }

    fn step_palette(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.matches(cx).len();
        if let Some(p) = &mut self.palette
            && n > 0
        {
            p.selected = (p.selected as isize + delta).rem_euclid(n as isize) as usize;
            cx.notify();
        }
    }

    pub(super) fn render_palette(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pal = self.palette.as_ref()?;
        let query = pal.input.read(cx).value().to_string();
        let jump = expand(&query);
        let hits = self.matches(cx);
        let rows: Vec<AnyElement> = if let Some(path) = jump {
            let ok = path.is_dir();
            vec![
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(10.0))
                    .h(px(32.0))
                    .rounded(px(6.0))
                    .bg(white_alpha(0.08))
                    .text_size(px(13.0))
                    .text_color(if ok { p.text_strong } else { p.text_muted })
                    .child(icon(
                        "folder",
                        14.0,
                        if ok { p.accent } else { p.text_muted },
                    ))
                    .child(div().flex_1().truncate().child(if ok {
                        format!("Go to {}", path.display())
                    } else {
                        format!("No folder at {}", path.display())
                    }))
                    .into_any_element(),
            ]
        } else {
            hits.iter()
                .take(12)
                .enumerate()
                .map(|(row, &ix)| {
                    let (name, label, keys, _) = COMMANDS[ix];
                    let on = row == pal.selected;
                    div()
                        .id(("cmd", ix))
                        .role(Role::ListBoxOption)
                        .aria_label(label)
                        .aria_selected(on)
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .px(px(10.0))
                        .h(px(32.0))
                        .rounded(px(6.0))
                        .text_size(px(13.0))
                        .cursor_pointer()
                        .text_color(if on { p.on_accent } else { p.text })
                        .when(on, |d| d.bg(p.accent))
                        .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.06))))
                        .on_click(cx.listener(move |this, _, w, cx| {
                            if let Some(p) = &mut this.palette {
                                p.selected = row;
                            }
                            this.run_palette(w, cx);
                        }))
                        .child(icon(
                            name,
                            14.0,
                            if on { p.on_accent } else { p.text_muted },
                        ))
                        .child(div().flex_1().child(label))
                        .when(!keys.is_empty(), |d| {
                            d.child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(if on { p.on_accent } else { p.text_muted })
                                    .child(keys),
                            )
                        })
                        .into_any_element()
                })
                .collect()
        };
        let empty = rows.is_empty();
        Some(
            div()
                .id("palette-overlay")
                .key_context("Dialog")
                .track_focus(&pal.focus)
                .on_action(cx.listener(|this, _: &CancelPrompt, w, cx| this.close_palette(w, cx)))
                .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                    match ev.keystroke.key.as_str() {
                        "up" => this.step_palette(-1, cx),
                        "down" => this.step_palette(1, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .pt(px(90.0))
                .bg(black_alpha(0.45))
                .on_click(cx.listener(|this, _, w, cx| this.close_palette(w, cx)))
                .child(
                    div()
                        .id("palette")
                        .role(Role::Dialog)
                        .aria_label("Command palette")
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .w(px(560.0))
                        .flex()
                        .flex_col()
                        .p(px(8.0))
                        .gap(px(6.0))
                        .rounded(px(14.0))
                        .bg(hex(0x242428).opacity(0.97))
                        .border_1()
                        .border_color(white_alpha(0.12))
                        .shadow(vec![BoxShadow {
                            color: black_alpha(0.7),
                            offset: point(px(0.0), px(24.0)),
                            blur_radius: px(60.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .h(px(40.0))
                                .px(px(10.0))
                                .border_b_1()
                                .border_color(white_alpha(0.08))
                                .child(icon("command", 15.0, p.text_muted))
                                .child(
                                    div().flex_1().child(
                                        Input::new(&pal.input)
                                            .appearance(false)
                                            .text_size(px(14.0))
                                            .text_color(p.text_strong),
                                    ),
                                ),
                        )
                        .children(rows)
                        .when(empty, |d| {
                            d.child(
                                div()
                                    .px(px(10.0))
                                    .py(px(8.0))
                                    .text_size(px(12.5))
                                    .text_color(p.text_muted)
                                    .child("No matching command"),
                            )
                        }),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::expand;

    #[test]
    fn expands_home_and_absolute_paths() {
        assert!(expand("open").is_none());
        let abs = std::env::temp_dir();
        assert_eq!(expand(&abs.to_string_lossy()), Some(abs.clone()));
        assert!(expand("relative/path").is_none());
        if std::env::var_os("HOME").is_some() {
            assert!(expand("~/x").unwrap().ends_with("x"));
        }
    }
}
