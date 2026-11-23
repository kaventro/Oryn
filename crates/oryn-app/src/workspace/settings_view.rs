use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::Workspace;
use crate::actions::*;
use crate::pane::ViewMode;
use crate::theme::{OrynTheme, Palette, TRAYS, black_alpha, hex, white_alpha, with_alpha};

#[derive(Clone, Copy)]
enum Toggle {
    ShowHidden,
    ConfirmDelete,
    UseTrash,
    Dual,
    Sidebar,
    CheckUpdates,
}

const TOGGLES: [(Toggle, &str, &str); 6] = [
    (
        Toggle::ShowHidden,
        "Show hidden files",
        "List files and folders whose names start with a dot.",
    ),
    (
        Toggle::ConfirmDelete,
        "Ask before deleting",
        "Show a confirmation before moving items to the Trash or deleting them.",
    ),
    (
        Toggle::UseTrash,
        "Move to Trash",
        "Delete sends items to the Trash. Turn off to delete them right away.",
    ),
    (
        Toggle::Dual,
        "Dual pane",
        "Show two file panes side by side.",
    ),
    (
        Toggle::Sidebar,
        "Sidebar",
        "Show favourites, drives and tags on the left.",
    ),
    (
        Toggle::CheckUpdates,
        "Check for updates",
        "Look for a newer Oryn release on GitHub when the app starts.",
    ),
];

impl Workspace {
    pub(super) fn open_settings(
        &mut self,
        _: &OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.settings_open = Some(focus);
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = None;
        self.focus_pane(self.active, window, cx);
    }

    fn toggle_value(&self, t: Toggle) -> bool {
        match t {
            Toggle::ShowHidden => self.settings.show_hidden,
            Toggle::ConfirmDelete => self.settings.confirm_delete,
            Toggle::UseTrash => self.settings.use_trash,
            Toggle::Dual => self.dual,
            Toggle::Sidebar => self.sidebar,
            Toggle::CheckUpdates => self.settings.check_updates,
        }
    }

    fn flip(&mut self, t: Toggle, window: &mut Window, cx: &mut Context<Self>) {
        match t {
            Toggle::ShowHidden => {
                let show = !self.settings.show_hidden;
                self.settings.show_hidden = show;
                for pane in self.tabs.iter().flatten().cloned().collect::<Vec<_>>() {
                    pane.update(cx, |p, cx| {
                        p.show_hidden = show;
                        p.refresh(None, cx);
                    });
                }
            }
            Toggle::ConfirmDelete => self.settings.confirm_delete = !self.settings.confirm_delete,
            Toggle::UseTrash => self.settings.use_trash = !self.settings.use_trash,
            Toggle::Dual => {
                self.toggle_panes(&TogglePanes, window, cx);
                if let Some(f) = &self.settings_open {
                    window.focus(f, cx);
                }
            }
            Toggle::Sidebar => self.toggle_sidebar(&ToggleSidebar, window, cx),
            Toggle::CheckUpdates => {
                self.settings.check_updates = !self.settings.check_updates;
                if self.settings.check_updates {
                    self.check_updates(cx);
                } else {
                    self.update = None;
                }
            }
        }
        self.persist(cx);
        cx.notify();
    }

    fn pick_tray(&mut self, tray: usize, cx: &mut Context<Self>) {
        cx.set_global(OrynTheme { tray });
        cx.refresh_windows();
        self.persist(cx);
    }

    fn pick_view(&mut self, view: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        self.set_view(view, window, cx);
        if let Some(f) = &self.settings_open {
            window.focus(f, cx);
        }
    }

    pub(super) fn render_settings(
        &self,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let focus = self.settings_open.as_ref()?;
        let current = cx.try_global::<OrynTheme>().map(|t| t.tray).unwrap_or(0) % TRAYS.len();
        let section = |label: &'static str| {
            div()
                .mt(px(16.0))
                .mb(px(8.0))
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_muted)
                .child(label.to_uppercase())
        };
        let swatches = TRAYS.iter().enumerate().map(|(ix, tray)| {
            let on = ix == current;
            let c = hex(tray.accent);
            div()
                .id(("tray", ix))
                .role(Role::RadioButton)
                .aria_label(tray.name)
                .aria_selected(on)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.0))
                .w(px(72.0))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.pick_tray(ix, cx)))
                .child(
                    div()
                        .size(px(30.0))
                        .rounded_full()
                        .bg(c)
                        .border_2()
                        .border_color(if on {
                            hex(0xffffff)
                        } else {
                            with_alpha(c, 0.4)
                        })
                        .shadow(vec![BoxShadow {
                            color: with_alpha(c, if on { 0.55 } else { 0.25 }),
                            offset: point(px(0.0), px(3.0)),
                            blur_radius: px(10.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }]),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_center()
                        .text_color(if on { p.text_strong } else { p.text_muted })
                        .child(tray.name),
                )
        });
        let views = [
            (ViewMode::List, "List"),
            (ViewMode::Grid, "Grid"),
            (ViewMode::Columns, "Columns"),
        ]
        .into_iter()
        .map(|(mode, label)| {
            let on = self.view == mode;
            div()
                .id(SharedString::from(format!("view-{label}")))
                .role(Role::RadioButton)
                .aria_label(label)
                .aria_selected(on)
                .flex_1()
                .flex()
                .justify_center()
                .py(px(5.0))
                .rounded(px(6.0))
                .text_size(px(12.5))
                .cursor_pointer()
                .text_color(if on { p.on_accent } else { p.text })
                .when(on, |d| d.bg(p.accent))
                .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.08))))
                .on_click(cx.listener(move |this, _, w, cx| this.pick_view(mode, w, cx)))
                .child(label)
        });
        let toggles = TOGGLES.iter().enumerate().map(|(ix, (t, label, hint))| {
            let t = *t;
            let on = self.toggle_value(t);
            div()
                .id(("toggle", ix))
                .role(Role::Switch)
                .aria_label(*label)
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .flex()
                .items_center()
                .gap(px(12.0))
                .py(px(8.0))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, w, cx| this.flip(t, w, cx)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(p.text_strong)
                                .child(*label),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.text_muted)
                                .child(*hint),
                        ),
                )
                .child(
                    div()
                        .w(px(36.0))
                        .h(px(20.0))
                        .p(px(2.0))
                        .rounded_full()
                        .flex()
                        .when(on, |d| d.justify_end().bg(p.accent))
                        .when(!on, |d| d.bg(white_alpha(0.16)))
                        .child(div().size(px(16.0)).rounded_full().bg(hex(0xffffff))),
                )
        });
        Some(
            div()
                .id("settings-overlay")
                .key_context("Dialog")
                .track_focus(focus)
                .on_action(cx.listener(|this, _: &CancelPrompt, w, cx| this.close_settings(w, cx)))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(black_alpha(0.55))
                .on_click(cx.listener(|this, _, w, cx| this.close_settings(w, cx)))
                .child(
                    div()
                        .id("settings")
                        .role(Role::Dialog)
                        .aria_label("Settings")
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .w(px(500.0))
                        .flex()
                        .flex_col()
                        .py(px(20.0))
                        .px(px(24.0))
                        .rounded(px(14.0))
                        .bg(hex(0x242428).opacity(0.97))
                        .border_1()
                        .border_color(white_alpha(0.12))
                        .shadow(vec![BoxShadow {
                            color: black_alpha(0.75),
                            offset: point(px(0.0), px(28.0)),
                            blur_radius: px(70.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .child(
                            div()
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_strong)
                                .child("Settings"),
                        )
                        .child(section("Accent"))
                        .child(div().flex().justify_between().children(swatches))
                        .child(section("View"))
                        .child(
                            div()
                                .flex()
                                .p(px(2.0))
                                .gap(px(2.0))
                                .rounded(px(8.0))
                                .bg(white_alpha(0.06))
                                .children(views),
                        )
                        .child(section("Behaviour"))
                        .children(toggles)
                        .child(
                            div()
                                .mt(px(16.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.text_muted)
                                        .child(concat!("Oryn ", env!("CARGO_PKG_VERSION"))),
                                )
                                .child(
                                    div()
                                        .id("settings-done")
                                        .role(Role::Button)
                                        .aria_label("Done")
                                        .px(px(16.0))
                                        .py(px(6.0))
                                        .rounded(px(7.0))
                                        .bg(p.accent)
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.on_accent)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(p.accent_hover))
                                        .on_click(
                                            cx.listener(|this, _, w, cx| {
                                                this.close_settings(w, cx)
                                            }),
                                        )
                                        .child("Done"),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
