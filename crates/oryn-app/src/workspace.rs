use std::path::{Path, PathBuf};

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::fmt_size;

use crate::actions::*;
use crate::icons::icon;
use crate::pane::FilePane;
use crate::terminal::TerminalView;
use crate::theme::{OrynTheme, Palette, TRAYS, palette, ui_font, with_alpha};

pub struct Workspace {
    panes: [Entity<FilePane>; 2],
    active: usize,
    terminal: Option<Entity<TerminalView>>,
    show_terminal: bool,
}

struct Place {
    label: &'static str,
    icon: &'static str,
    path: PathBuf,
}

fn places() -> Vec<Place> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let mut v = vec![Place {
        label: "Home",
        icon: "house",
        path: home.clone(),
    }];
    for (label, icon, sub) in [
        ("Desktop", "monitor", "Desktop"),
        ("Documents", "file-text", "Documents"),
        ("Downloads", "download", "Downloads"),
    ] {
        let p = home.join(sub);
        if p.is_dir() {
            v.push(Place {
                label,
                icon,
                path: p,
            });
        }
    }
    v.push(Place {
        label: "File System",
        icon: "hard-drive",
        path: PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }),
    });
    v
}

impl Workspace {
    pub fn new(left: PathBuf, right: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = cx.new(|cx| FilePane::new(left, cx));
        let r = cx.new(|cx| FilePane::new(right, cx));
        cx.observe(&l, |_, _, cx| cx.notify()).detach();
        cx.observe(&r, |_, _, cx| cx.notify()).detach();
        let focus = l.read(cx).focus.clone();
        window.focus(&focus, cx);
        Self {
            panes: [l, r],
            active: 0,
            terminal: None,
            show_terminal: false,
        }
    }

    pub fn left(&self) -> Entity<FilePane> {
        self.panes[0].clone()
    }

    fn active_pane(&self) -> &Entity<FilePane> {
        &self.panes[self.active]
    }

    fn focus_pane(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active = ix;
        let f = self.panes[ix].read(cx).focus.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    fn switch_pane(&mut self, _: &SwitchPane, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_pane(1 - self.active, window, cx);
    }

    fn toggle_terminal(&mut self, _: &ToggleTerminal, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_terminal {
            self.show_terminal = false;
            self.focus_pane(self.active, window, cx);
            return;
        }
        let cwd = self.active_pane().read(cx).path.clone();
        let term = self
            .terminal
            .get_or_insert_with(|| cx.new(|cx| TerminalView::new(&cwd, cx)))
            .clone();
        self.show_terminal = true;
        let f = term.read(cx).focus.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    fn cycle_tray(&mut self, _: &CycleTray, _: &mut Window, cx: &mut Context<Self>) {
        let next = cx
            .try_global::<OrynTheme>()
            .map(|t| t.tray + 1)
            .unwrap_or(1)
            % TRAYS.len();
        cx.set_global(OrynTheme { tray: next });
        cx.refresh_windows();
    }

    fn go_to(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let pane = self.active_pane().clone();
        pane.update(cx, |p, cx| p.navigate(path, cx));
        self.focus_pane(self.active, window, cx);
    }

    fn render_sidebar(
        &self,
        current: &Path,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let items = places().into_iter().enumerate().map(|(i, place)| {
            let selected = current == place.path;
            let target = place.path.clone();
            div()
                .id(("place", i))
                .role(Role::Link)
                .aria_label(SharedString::from(format!("Open {}", place.label)))
                .flex()
                .items_center()
                .gap(px(10.0))
                .h(px(30.0))
                .px(px(10.0))
                .rounded(px(6.0))
                .text_size(px(13.0))
                .text_color(if selected {
                    p.text
                } else {
                    with_alpha(p.text, 0.82)
                })
                .when(selected, |d| d.bg(p.accent_soft))
                .when(!selected, |d| d.hover(|s| s.bg(p.hover)))
                .cursor_pointer()
                .on_click(
                    cx.listener(move |this, _, window, cx| this.go_to(target.clone(), window, cx)),
                )
                .child(icon(
                    place.icon,
                    15.0,
                    if selected { p.accent } else { p.text_muted },
                ))
                .child(place.label)
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(208.0))
            .h_full()
            .px(px(10.0))
            .pt(px(18.0))
            .bg(p.sidebar)
            .border_r_1()
            .border_color(p.divider)
            .child(
                div()
                    .px(px(10.0))
                    .pb(px(18.0))
                    .text_size(px(17.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(p.text)
                    .child("Oryn"),
            )
            .child(
                div()
                    .px(px(10.0))
                    .pb(px(6.0))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_faint)
                    .child("PLACES"),
            )
            .child(div().flex().flex_col().gap(px(2.0)).children(items))
    }

    fn tool_button(
        id: &'static str,
        name: &'static str,
        label: &'static str,
        active: bool,
        p: &Palette,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex()
            .items_center()
            .justify_center()
            .size(px(32.0))
            .rounded(px(8.0))
            .border_1()
            .cursor_pointer()
            .when(active, |d| d.bg(p.accent).border_color(p.accent))
            .when(!active, |d| {
                d.bg(p.card)
                    .border_color(p.card_border)
                    .hover(|s| s.bg(with_alpha(p.card, 0.7)))
            })
            .child(icon(name, 16.0, if active { p.on_accent } else { p.text }))
    }

    fn render_top(
        &self,
        title: SharedString,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tray = p.tray_name.clone();
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(56.0))
            .px(px(20.0))
            .gap(px(8.0))
            .child(
                div()
                    .id("title")
                    .flex_1()
                    .role(Role::Heading)
                    .text_size(px(20.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(p.text)
                    .truncate()
                    .child(title),
            )
            .child(
                div()
                    .px(px(10.0))
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .rounded(px(12.0))
                    .bg(p.accent_soft)
                    .text_size(px(11.5))
                    .text_color(p.accent)
                    .child(tray),
            )
            .child(
                Self::tool_button("tray", "palette", "Change accent colour", false, p)
                    .on_click(cx.listener(|this, _, w, cx| this.cycle_tray(&CycleTray, w, cx))),
            )
            .child(
                Self::tool_button(
                    "term",
                    "square-terminal",
                    "Toggle terminal",
                    self.show_terminal,
                    p,
                )
                .on_click(
                    cx.listener(|this, _, w, cx| this.toggle_terminal(&ToggleTerminal, w, cx)),
                ),
            )
    }

    fn render_path_bar(
        &self,
        path: &Path,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut acc = PathBuf::new();
        let mut crumbs = Vec::new();
        for (i, comp) in path.components().enumerate() {
            acc.push(comp.as_os_str());
            let label = match comp {
                std::path::Component::RootDir => "File System".to_string(),
                other => other.as_os_str().to_string_lossy().to_string(),
            };
            crumbs.push((i, label, acc.clone()));
        }
        let last = crumbs.len().saturating_sub(1);
        let children = crumbs.into_iter().flat_map(|(i, label, target)| {
            let is_last = i == last;
            let mut out: Vec<AnyElement> = Vec::new();
            if i > 0 {
                out.push(icon("chevron-right", 12.0, p.text_faint).into_any_element());
            }
            out.push(
                div()
                    .id(("crumb", i))
                    .role(Role::Link)
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(5.0))
                    .text_size(px(12.5))
                    .text_color(if is_last { p.text } else { p.text_muted })
                    .cursor_pointer()
                    .hover(|s| s.bg(p.hover))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.go_to(target.clone(), window, cx)
                    }))
                    .child(label)
                    .into_any_element(),
            );
            out
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.0))
            .mx(px(20.0))
            .mb(px(10.0))
            .h(px(32.0))
            .px(px(8.0))
            .rounded(px(8.0))
            .bg(p.card)
            .border_1()
            .border_color(p.card_border)
            .overflow_hidden()
            .child(
                div()
                    .pl(px(4.0))
                    .pr(px(4.0))
                    .child(icon("folder", 14.0, p.text_muted)),
            )
            .children(children)
    }

    fn render_status(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let pane = self.active_pane().read(cx);
        let (n, bytes) = pane.marked_summary();
        let total = pane.item_count();
        let left = if n > 0 {
            format!("{n} of {total} selected ({})", fmt_size(Some(bytes)))
        } else {
            format!("{total} items")
        };
        let path = pane.path.display().to_string();
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(30.0))
            .px(px(20.0))
            .border_t_1()
            .border_color(p.divider)
            .text_size(px(11.5))
            .text_color(p.text_muted)
            .child(div().flex_1().child(left))
            .child(
                div()
                    .id("status-term")
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(10.0))
                    .h(px(20.0))
                    .rounded(px(10.0))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.card_border)
                    .cursor_pointer()
                    .on_click(
                        cx.listener(|this, _, w, cx| this.toggle_terminal(&ToggleTerminal, w, cx)),
                    )
                    .child(icon("terminal", 12.0, p.text_muted))
                    .child("Terminal")
                    .child(div().text_color(p.text_faint).child("Ctrl+`")),
            )
            .child(div().flex_1().flex().justify_end().truncate().child(path))
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for i in 0..2 {
            if self.panes[i].read(cx).focus.contains_focused(window, cx) {
                self.active = i;
            }
        }
        let p = palette(cx);
        let (title, path) = {
            let a = self.active_pane().read(cx);
            (a.title(), a.path.clone())
        };
        let terminal = self.show_terminal.then(|| self.terminal.clone()).flatten();

        div()
            .id("workspace")
            .key_context("Workspace")
            .role(Role::Application)
            .aria_label("Oryn")
            .on_action(cx.listener(Self::switch_pane))
            .on_action(cx.listener(Self::toggle_terminal))
            .on_action(cx.listener(Self::cycle_tray))
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .flex()
            .size_full()
            .bg(p.bg)
            .font_family(ui_font())
            .text_color(p.text)
            .child(self.render_sidebar(&path, &p, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.render_top(title, &p, cx))
                    .child(self.render_path_bar(&path, &p, cx))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .gap(px(10.0))
                            .px(px(20.0))
                            .pb(px(12.0))
                            .child(self.panes[0].clone())
                            .child(self.panes[1].clone()),
                    )
                    .when_some(terminal, |d, t| {
                        d.child(
                            div()
                                .flex_none()
                                .h(px(250.0))
                                .mx(px(20.0))
                                .mb(px(12.0))
                                .rounded(px(10.0))
                                .border_1()
                                .border_color(p.card_border)
                                .overflow_hidden()
                                .child(t),
                        )
                    })
                    .child(self.render_status(&p, cx)),
            )
    }
}
