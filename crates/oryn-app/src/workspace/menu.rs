use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::batch::BatchKind;

use super::Workspace;
use super::dialog::{Choice, Dialog};
use crate::actions::*;
use crate::icons::icon;
use crate::pane::{FilePane, PaneEvent};
use crate::theme::{Palette, black_alpha, hex, white_alpha, with_alpha};

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

fn entries(on_item: bool) -> Vec<Entry> {
    let mut v = Vec::new();
    if on_item {
        v.extend([
            item("folder-open", "Open", "Enter", OpenEntry),
            Entry::Separator,
            item("copy", "Copy to Other Pane", "F5", CopyToOther),
            item("folder-input", "Move to Other Pane", "F6", MoveToOther),
            item("copy-plus", "Duplicate", "\u{21e7}F5", DuplicateItems),
            item("pencil", "Rename", "F2", RenameItem),
            item("text-cursor-input", "Batch Rename\u{2026}", "", MultiRename),
            item("clipboard", "Copy Path", "", CopyPath),
            item("tag", "Tags\u{2026}", "", ShowTags),
            item("hash", "Checksums", "", ShowChecksums),
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
    v.extend([
        item("folder-plus", "New Folder", "F7", NewFolder),
        item("file-plus", "New File", "\u{21e7}F7", NewFile),
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
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                self.menu = Some(Menu {
                    position: *position,
                    entries: entries(*on_item),
                    selected: None,
                    focus,
                });
                cx.notify();
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
