use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::batch::BatchKind;

use super::Workspace;
use super::dialog::{Choice, Dialog};
use crate::actions::*;
use crate::icons::icon;
use crate::pane::{FilePane, PaneEvent, ViewMode};
use crate::theme::{Palette, black_alpha, hex, white_alpha, with_alpha};
use oryn_core::ui::arrange::SortField;

pub struct Menu {
    position: Point<Pixels>,
    entries: Vec<Entry>,
    selected: Option<usize>,
    focus: FocusHandle,
}

enum Entry {
    Item {
        icon: &'static str,
        label: &'static str,
        keys: &'static str,
        danger: bool,
        action: Box<dyn Action>,
    },
    Separator,
}

fn item(icon: &'static str, label: &'static str, keys: &'static str, action: impl Action) -> Entry {
    Entry::Item {
        icon,
        label,
        keys,
        danger: false,
        action: Box::new(action),
    }
}

fn danger(
    icon: &'static str,
    label: &'static str,
    keys: &'static str,
    action: impl Action,
) -> Entry {
    Entry::Item {
        icon,
        label,
        keys,
        danger: true,
        action: Box::new(action),
    }
}

pub(super) struct MenuCtx {
    pub on_item: bool,
    pub on_dir: bool,
    pub archive: bool,
    pub inside_archive: bool,
    pub favorite: bool,
    pub can_paste: bool,
}

fn entries(m: &MenuCtx) -> Vec<Entry> {
    let mut v = Vec::new();
    if m.on_item {
        v.push(item("folder-open", "Open", "Enter", OpenEntry));
        if m.on_dir && !m.inside_archive {
            v.push(if m.favorite {
                item("star-off", "Remove from Favourites", "", AddFavorite)
            } else {
                item("star", "Add to Favourites", "", AddFavorite)
            });
            v.push(item(
                "square-terminal",
                "Open in Terminal",
                "",
                OpenInTerminal,
            ));
        }
        v.push(Entry::Separator);
        if m.inside_archive {
            v.extend([
                item("copy", "Copy to Other Pane", "F5", CopyToOther),
                item("copy", "Copy", "", CopyItems),
                item("clipboard", "Copy Path", "", CopyPath),
                Entry::Separator,
            ]);
        } else {
            v.extend([
                item("scissors", "Cut", "", CutItems),
                item("copy", "Copy", "", CopyItems),
            ]);
            if m.can_paste {
                v.push(item("clipboard-paste", "Paste", "", PasteItems));
            }
            v.extend([
                Entry::Separator,
                item("copy", "Copy to Other Pane", "F5", CopyToOther),
                item("folder-input", "Move to Other Pane", "F6", MoveToOther),
                item("copy-plus", "Duplicate", "\u{21e7}F5", DuplicateItems),
                item("pencil", "Rename", "F2", RenameItem),
                item("text-cursor-input", "Batch Rename\u{2026}", "", MultiRename),
                item("file-archive", "Compress to ZIP", "", CompressZip),
            ]);
            if m.archive {
                v.push(item("package-open", "Extract Here", "", ExtractHere));
            }
            v.extend([
                item("clipboard", "Copy Path", "", CopyPath),
                item("tag", "Tags\u{2026}", "", ShowTags),
                item("hash", "Checksums", "", ShowChecksums),
                item("info", "Properties", "Alt+Enter", ShowProperties),
                Entry::Separator,
                danger("trash", "Move to Trash", "F8", DeleteItems),
                danger(
                    "archive-x",
                    "Delete Permanently",
                    "\u{21e7}Del",
                    DeletePermanently,
                ),
                Entry::Separator,
            ]);
        }
    }
    if !m.inside_archive {
        v.extend([
            item("folder-plus", "New Folder", "F7", NewFolder),
            item("file-plus", "New File", "\u{21e7}F7", NewFile),
        ]);
        if m.can_paste && !m.on_item {
            v.push(item("clipboard-paste", "Paste", "", PasteItems));
        }
        if !m.on_item {
            v.push(if m.favorite {
                item(
                    "star-off",
                    "Remove Folder from Favourites",
                    "",
                    FavoriteThisFolder,
                )
            } else {
                item("star", "Add Folder to Favourites", "", FavoriteThisFolder)
            });
            v.push(item(
                "square-terminal",
                "Open in Terminal",
                "",
                OpenInTerminal,
            ));
        }
    }
    v.extend([
        item(
            "list-checks",
            "Select by Mask\u{2026}",
            "Alt+S",
            SelectByMask,
        ),
        item("refresh-cw", "Refresh", "", RefreshPanes),
    ]);
    v
}

pub(super) struct MoreCtx {
    pub view: ViewMode,
    pub sort: SortField,
    pub hidden: bool,
    pub favorite: bool,
}

fn more_entries(m: &MoreCtx) -> Vec<Entry> {
    let mark = |on: bool, icon: &'static str| if on { "check" } else { icon };
    vec![
        item(
            mark(m.view == ViewMode::List, "files"),
            "List View",
            "",
            ViewList,
        ),
        item(
            mark(m.view == ViewMode::Grid, "files"),
            "Grid View",
            "",
            ViewGrid,
        ),
        item(
            mark(m.view == ViewMode::Columns, "files"),
            "Column View",
            "",
            ViewColumns,
        ),
        Entry::Separator,
        item(
            mark(m.sort == SortField::Name, "arrow-up-down"),
            "Sort by Name",
            "",
            SortByName,
        ),
        item(
            mark(m.sort == SortField::Date, "arrow-up-down"),
            "Sort by Date Modified",
            "",
            SortByDate,
        ),
        item(
            mark(m.sort == SortField::Size, "arrow-up-down"),
            "Sort by Size",
            "",
            SortBySize,
        ),
        item(
            mark(m.hidden, "eye-off"),
            "Show Hidden Files",
            "",
            ToggleHidden,
        ),
        Entry::Separator,
        item("folder-plus", "New Folder", "F7", NewFolder),
        item("file-plus", "New File", "\u{21e7}F7", NewFile),
        if m.favorite {
            item(
                "star-off",
                "Remove Folder from Favourites",
                "",
                FavoriteThisFolder,
            )
        } else {
            item("star", "Add Folder to Favourites", "", FavoriteThisFolder)
        },
        item("square-terminal", "Open in Terminal", "", OpenInTerminal),
        item("info", "Properties", "", ShowProperties),
        item("clipboard", "Copy Path", "", CopyPath),
        Entry::Separator,
        item("chart-pie", "Disk Space\u{2026}", "", ShowDiskSpace),
        item("files", "Find Duplicates\u{2026}", "", ShowDuplicates),
        item("arrow-left-right", "Swap Panes", "Ctrl+U", SwapPanes),
        item("folder-open", "Go to Folder\u{2026}", "Ctrl+L", GoToPath),
    ]
}

impl Workspace {
    pub(super) fn on_pane_event(
        &mut self,
        pane: &Entity<FilePane>,
        event: &PaneEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(side) = (0..2).find(|s| self.pane(*s) == pane) {
            self.active = side;
        }
        match event {
            PaneEvent::ContextMenu(position, on_item) => {
                let ctx = self.menu_ctx(pane, *on_item, cx);
                self.show_menu(*position, entries(&ctx), window, cx);
            }
            PaneEvent::Drop { paths, dir } => {
                if self.job.is_some() {
                    return;
                }
                let sources: Vec<_> = paths
                    .iter()
                    .filter(|p| p.parent() != Some(dir.as_path()))
                    .cloned()
                    .collect();
                if sources.is_empty() {
                    return;
                }
                let kind = if window.modifiers().shift {
                    BatchKind::Move
                } else {
                    BatchKind::Copy
                };
                let value = dir.to_string_lossy().into_owned();
                self.open_dialog(
                    Dialog::Transfer {
                        kind,
                        sources,
                        conflicts: None,
                    },
                    &value,
                    window,
                    cx,
                );
                self.choose(Choice::Confirm, window, cx);
            }
        }
    }

    fn menu_ctx(&self, pane: &Entity<FilePane>, on_item: bool, cx: &App) -> MenuCtx {
        let pane = pane.read(cx);
        let inside_archive = pane.in_archive();
        let cursor = pane.cursor_entry();
        let on_dir = on_item && cursor.as_ref().is_some_and(|(_, d)| *d);
        let fav_target = if on_dir {
            cursor.map(|(n, _)| pane.op_dir().join(n))
        } else if on_item {
            None
        } else {
            Some(pane.path.clone())
        };
        MenuCtx {
            on_item,
            on_dir,
            archive: pane
                .targets()
                .iter()
                .any(|t| oryn_core::batch::is_archive(t)),
            inside_archive,
            favorite: fav_target.is_some_and(|p| self.favorites.contains(&p)),
            can_paste: self.clipboard.is_some(),
        }
    }

    fn show_menu(
        &mut self,
        position: Point<Pixels>,
        entries: Vec<Entry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.menu = Some(Menu {
            position,
            entries,
            selected: None,
            focus,
        });
        cx.notify();
    }

    pub(super) fn open_more_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.active_pane().read(cx);
        let ctx = MoreCtx {
            view: self.view,
            sort: pane.order.field,
            hidden: self.settings.show_hidden,
            favorite: self.favorites.contains(&pane.path),
        };
        self.show_menu(position, more_entries(&ctx), window, cx);
    }

    pub(super) fn open_context_menu(
        &mut self,
        _: &OpenContextMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.active_pane().clone();
        let on_item = pane.read(cx).cursor_entry().is_some();
        let b = window.bounds();
        let position = point(b.size.width / 2.0 - px(110.0), b.size.height / 3.0);
        self.on_pane_event(
            &pane,
            &PaneEvent::ContextMenu(position, on_item),
            window,
            cx,
        );
        if let Some(menu) = &mut self.menu {
            menu.selected = menu
                .entries
                .iter()
                .position(|e| matches!(e, Entry::Item { .. }));
        }
    }

    fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.focus_pane(self.active, window, cx);
    }

    fn run_entry(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let action = match self.menu.as_ref().and_then(|m| m.entries.get(ix)) {
            Some(Entry::Item { action, .. }) => action.boxed_clone(),
            _ => return,
        };
        self.close_menu(window, cx);
        window.dispatch_action(action, cx);
    }

    fn step_menu(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(menu) = &mut self.menu else {
            return;
        };
        let items: Vec<usize> = menu
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Entry::Item { .. }))
            .map(|(i, _)| i)
            .collect();
        if items.is_empty() {
            return;
        }
        let pos = menu
            .selected
            .and_then(|s| items.iter().position(|i| *i == s));
        let next = match pos {
            None if delta > 0 => 0,
            None => items.len() - 1,
            Some(p) => (p as isize + delta).rem_euclid(items.len() as isize) as usize,
        };
        menu.selected = Some(items[next]);
        cx.notify();
    }

    pub(super) fn copy_path(&mut self, _: &CopyPath, _: &mut Window, cx: &mut Context<Self>) {
        let paths = self.active_pane().read(cx).targets();
        let text = if paths.is_empty() {
            self.active_pane()
                .read(cx)
                .path
                .to_string_lossy()
                .into_owned()
        } else {
            paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("\n")
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.active_pane().clone().update(cx, |p, cx| {
            p.report("Copied the path to the clipboard.".into(), cx)
        });
    }

    #[cfg(test)]
    pub(super) fn has_menu_item(&self, label: &str) -> bool {
        self.menu.as_ref().is_some_and(|m| {
            m.entries
                .iter()
                .any(|e| matches!(e, Entry::Item { label: l, .. } if *l == label))
        })
    }

    pub(super) fn render_menu(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu.as_ref()?;
        let rows = menu
            .entries
            .iter()
            .enumerate()
            .map(|(ix, entry)| match entry {
                Entry::Separator => div()
                    .h(px(1.0))
                    .my(px(4.0))
                    .mx(px(6.0))
                    .bg(white_alpha(0.08))
                    .into_any_element(),
                Entry::Item {
                    icon: name,
                    label,
                    keys,
                    danger,
                    ..
                } => {
                    let selected = menu.selected == Some(ix);
                    let hot = if *danger { hex(0xff6961) } else { p.accent };
                    let base = if *danger { hex(0xff6961) } else { p.text };
                    div()
                        .id(("menu-item", ix))
                        .debug_selector(|| format!("menu-{label}"))
                        .role(Role::MenuItem)
                        .aria_label(*label)
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .text_size(px(12.5))
                        .text_color(if selected { hot } else { base })
                        .cursor_pointer()
                        .when(selected, |d| d.bg(white_alpha(0.08)))
                        .hover(move |s| s.bg(white_alpha(0.08)).text_color(hot))
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if let Some(m) = &mut this.menu
                                && m.selected != Some(ix)
                            {
                                m.selected = Some(ix);
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |this, _, w, cx| this.run_entry(ix, w, cx)))
                        .child(icon(name, 13.0, if selected { hot } else { p.text_muted }))
                        .child(div().flex_1().truncate().child(*label))
                        .when(!keys.is_empty(), |d| {
                            d.child(
                                div()
                                    .pl(px(10.0))
                                    .text_size(px(11.0))
                                    .text_color(if selected { hot } else { p.text_muted })
                                    .child(*keys),
                            )
                        })
                        .into_any_element()
                }
            });
        Some(
            div()
                .id("menu-overlay")
                .key_context("ContextMenu")
                .track_focus(&menu.focus)
                .on_action(cx.listener(|this, _: &CancelPrompt, w, cx| this.close_menu(w, cx)))
                .on_action(cx.listener(|this, _: &MoveUp, _, cx| this.step_menu(-1, cx)))
                .on_action(cx.listener(|this, _: &MoveDown, _, cx| this.step_menu(1, cx)))
                .on_action(cx.listener(|this, _: &ConfirmPrompt, w, cx| {
                    if let Some(ix) = this.menu.as_ref().and_then(|m| m.selected) {
                        this.run_entry(ix, w, cx);
                    }
                }))
                .occlude()
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, w, cx| this.close_menu(w, cx)),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, w, cx| this.close_menu(w, cx)),
                )
                .child(deferred(
                    anchored()
                        .position(menu.position)
                        .snap_to_window_with_margin(px(8.0))
                        .child(
                            div()
                                .id("menu")
                                .role(Role::Menu)
                                .aria_label("Context menu")
                                .occlude()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .min_w(px(230.0))
                                .max_w(px(290.0))
                                .p(px(4.0))
                                .flex()
                                .flex_col()
                                .rounded(px(10.0))
                                .bg(hex(0x1e1e22).opacity(0.97))
                                .border_1()
                                .border_color(with_alpha(p.accent, 0.15))
                                .shadow(vec![BoxShadow {
                                    color: black_alpha(0.5),
                                    offset: point(px(0.0), px(12.0)),
                                    blur_radius: px(36.0),
                                    spread_radius: px(0.0),
                                    inset: false,
                                }])
                                .children(rows),
                        ),
                ))
                .into_any_element(),
        )
    }
}
