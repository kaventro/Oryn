use std::path::{Path, PathBuf};

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::fmt_size;

use crate::actions::*;
use crate::icons::{icon, oryn_icon};
use crate::pane::FilePane;
use crate::terminal::TerminalView;
use crate::theme::{
    OrynTheme, Palette, TRAYS, black_alpha, hex, palette, ui_font, white_alpha, with_alpha,
};

const TRAFFIC_LIGHTS: f32 = 78.0;

pub struct Workspace {
    panes: [Entity<FilePane>; 2],
    active: usize,
    dual: bool,
    sidebar: bool,
    terminal: Option<Entity<TerminalView>>,
    show_terminal: bool,
    search: Entity<InputState>,
    placeholder_for: SharedString,
}

struct Place {
    label: &'static str,
    icon: &'static str,
    path: PathBuf,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn favorites() -> Vec<Place> {
    let home = home_dir();
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
    v
}

fn system_drive() -> Place {
    Place {
        label: if cfg!(target_os = "macos") {
            "Macintosh HD"
        } else if cfg!(windows) {
            "Local Disk (C:)"
        } else {
            "File System"
        },
        icon: "hard-drive",
        path: PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" }),
    }
}

const TAGS: [(&str, u32); 7] = [
    ("Red", 0xff453a),
    ("Orange", 0xff9f0a),
    ("Yellow", 0xffd60a),
    ("Green", 0x34c759),
    ("Blue", 0x0a84ff),
    ("Purple", 0xbf5af2),
    ("Gray", 0x8e8e93),
];

impl Workspace {
    pub fn new(left: PathBuf, right: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = cx.new(|cx| FilePane::new(left, cx));
        let r = cx.new(|cx| FilePane::new(right, cx));
        cx.observe(&l, |_, _, cx| cx.notify()).detach();
        cx.observe(&r, |_, _, cx| cx.notify()).detach();
        let search = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe(&search, |this, state, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                let pane = this.panes[this.active].clone();
                pane.update(cx, |p, cx| p.set_filter(&text, cx));
            }
        })
        .detach();
        let focus = l.read(cx).focus.clone();
        window.focus(&focus, cx);
        Self {
            panes: [l, r],
            active: 0,
            dual: true,
            sidebar: true,
            terminal: None,
            show_terminal: false,
            search,
            placeholder_for: SharedString::default(),
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
        if self.dual {
            self.focus_pane(1 - self.active, window, cx);
        }
    }

    fn toggle_panes(&mut self, _: &TogglePanes, window: &mut Window, cx: &mut Context<Self>) {
        self.dual = !self.dual;
        for pane in &self.panes {
            let dual = self.dual;
            pane.update(cx, |p, _| p.dual = dual);
        }
        if !self.dual {
            self.focus_pane(0, window, cx);
        }
        cx.notify();
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar = !self.sidebar;
        cx.notify();
    }

    fn focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |s, cx| s.focus(window, cx));
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

    fn with_pane(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut FilePane, &mut Context<FilePane>),
    ) {
        let pane = self.active_pane().clone();
        pane.update(cx, f);
        self.focus_pane(self.active, window, cx);
    }

    fn section_header(title: &'static str, plus: bool, p: &Palette) -> Div {
        div()
            .flex()
            .items_center()
            .h(px(28.0))
            .pl(px(8.0))
            .pr(px(12.0))
            .mt(px(10.0))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap(px(5.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_muted)
                    .child(oryn_icon("section-chevron", 10.0, p.text_muted))
                    .child(title),
            )
            .when(plus, |d| {
                d.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(p.text_muted)
                        .child("+"),
                )
            })
    }

    fn sidebar_item(
        id: (&'static str, usize),
        label: &'static str,
        lead: AnyElement,
        selected: bool,
        p: &Palette,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(Role::Link)
            .aria_label(label)
            .flex()
            .items_center()
            .gap(px(10.0))
            .h(px(30.0))
            .px(px(12.0))
            .rounded(px(6.0))
            .text_size(px(13.0))
            .text_color(if selected {
                p.text_strong
            } else {
                p.text_sidebar
            })
            .when(selected, |d| d.bg(white_alpha(0.08)))
            .when(!selected, |d| d.hover(|s| s.bg(white_alpha(0.05))))
            .cursor_pointer()
            .child(lead)
            .child(label)
    }

    fn render_sidebar(
        &self,
        current: &Path,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let favs = favorites().into_iter().enumerate().map(|(i, place)| {
            let target = place.path.clone();
            Self::sidebar_item(
                ("fav", i),
                place.label,
                icon(place.icon, 16.0, p.text_sidebar).into_any_element(),
                current == place.path,
                p,
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.go_to(target.clone(), window, cx)),
            )
        });
        let drive = system_drive();
        let drive_target = drive.path.clone();
        let drive_item = Self::sidebar_item(
            ("drive", 0),
            drive.label,
            icon(drive.icon, 16.0, p.accent).into_any_element(),
            current == drive.path,
            p,
        )
        .on_click(
            cx.listener(move |this, _, window, cx| this.go_to(drive_target.clone(), window, cx)),
        );
        let tags = TAGS.iter().enumerate().map(|(i, (label, color))| {
            Self::sidebar_item(
                ("tag", i),
                label,
                div()
                    .size(px(9.0))
                    .rounded_full()
                    .bg(hex(*color))
                    .mx(px(3.5))
                    .into_any_element(),
                false,
                p,
            )
        });

        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(220.0))
            .h_full()
            .bg(p.bg)
            .border_r_1()
            .border_color(p.divider)
            .child(
                div()
                    .id("sidebar-top")
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(52.0))
                    .pl(px(if cfg!(target_os = "macos") {
                        TRAFFIC_LIGHTS
                    } else {
                        16.0
                    }))
                    .pr(px(16.0))
                    .child(self.brand(p))
                    .child(
                        div()
                            .id("sidebar-toggle")
                            .role(Role::Button)
                            .aria_label("Hide sidebar")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(28.0))
                            .rounded(px(6.0))
                            .bg(white_alpha(0.04))
                            .border_1()
                            .border_color(white_alpha(0.06))
                            .cursor_pointer()
                            .hover(|s| s.bg(white_alpha(0.1)))
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.toggle_sidebar(&ToggleSidebar, w, cx)
                            }))
                            .child(oryn_icon("sidebar-toggle", 16.0, p.text_muted)),
                    ),
            )
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(8.0))
                    .child(Self::section_header("FAVORITES", true, p))
                    .children(favs)
                    .child(Self::section_header("LOCATIONS", false, p))
                    .child(drive_item)
                    .child(Self::section_header("REMOTE (SFTP)", true, p))
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(6.0))
                            .text_size(px(11.0))
                            .text_color(p.text_muted)
                            .child("No saved servers"),
                    )
                    .child(Self::section_header("TAGS", true, p))
                    .children(tags),
            )
    }

    fn brand(&self, p: &Palette) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(28.0))
            .px(px(if cfg!(target_os = "macos") {
                13.0
            } else {
                20.0
            }))
            .rounded_full()
            .bg(white_alpha(0.08))
            .border_1()
            .border_color(white_alpha(0.14))
            .shadow(vec![
                BoxShadow {
                    color: black_alpha(0.45),
                    offset: point(px(0.0), px(1.0)),
                    blur_radius: px(2.0),
                    spread_radius: px(0.0),
                    inset: false,
                },
                BoxShadow {
                    color: with_alpha(p.accent, 0.2),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(18.0),
                    spread_radius: px(0.0),
                    inset: false,
                },
            ])
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(white_alpha(0.92))
                    .child("O r y n"),
            )
    }

    fn icon_button(
        id: &'static str,
        name: &'static str,
        label: impl Into<SharedString>,
        active: bool,
        p: &Palette,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(32.0))
            .rounded(px(6.0))
            .border_1()
            .cursor_pointer()
            .when(active, |d| {
                d.bg(p.accent)
                    .border_color(p.accent)
                    .shadow(vec![BoxShadow {
                        color: with_alpha(p.accent, 0.35),
                        offset: point(px(0.0), px(2.0)),
                        blur_radius: px(8.0),
                        spread_radius: px(0.0),
                        inset: false,
                    }])
            })
            .when(!active, |d| {
                d.bg(white_alpha(0.06))
                    .border_color(p.control_border)
                    .hover(|s| s.bg(white_alpha(0.12)))
            })
            .child(oryn_icon(
                name,
                15.0,
                if active {
                    p.on_accent
                } else {
                    p.text_secondary
                },
            ))
    }

    fn render_header(
        &self,
        title: SharedString,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let caption = !cfg!(target_os = "macos");
        div()
            .id("header")
            .window_control_area(WindowControlArea::Drag)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.0))
            .h(px(52.0))
            .pl(px(if !self.sidebar && cfg!(target_os = "macos") {
                TRAFFIC_LIGHTS
            } else {
                20.0
            }))
            .pr(px(if caption { 8.0 } else { 20.0 }))
            .when(!self.sidebar, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(self.brand(p))
                        .child(
                            div()
                                .id("sidebar-show")
                                .role(Role::Button)
                                .aria_label("Show sidebar")
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(28.0))
                                .rounded(px(6.0))
                                .bg(white_alpha(0.04))
                                .border_1()
                                .border_color(white_alpha(0.06))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, w, cx| {
                                    this.toggle_sidebar(&ToggleSidebar, w, cx)
                                }))
                                .child(oryn_icon("sidebar-toggle", 16.0, p.text_muted)),
                        ),
                )
            })
            .child(
                div()
                    .id("folder-title")
                    .role(Role::Heading)
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(p.text_strong)
                    .child(title.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .rounded(px(6.0))
                    .bg(p.accent)
                    .border_1()
                    .border_color(p.accent)
                    .shadow(vec![BoxShadow {
                        color: with_alpha(p.accent, 0.35),
                        offset: point(px(0.0), px(2.0)),
                        blur_radius: px(8.0),
                        spread_radius: px(0.0),
                        inset: false,
                    }])
                    .text_size(px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.on_accent)
                    .child(icon("folder", 13.0, p.on_accent))
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(Self::icon_button("add-tab", "add-tab", "New tab", false, p))
                    .child(
                        Self::icon_button(
                            "pane-toggle",
                            "pane-toggle",
                            "Toggle dual pane",
                            self.dual,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.toggle_panes(&TogglePanes, w, cx)),
                        ),
                    )
                    .child(Self::icon_button(
                        "remote",
                        "remote",
                        "Remote servers",
                        false,
                        p,
                    ))
                    .child(
                        Self::icon_button(
                            "terminal",
                            "terminal",
                            "Toggle terminal",
                            self.show_terminal,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| {
                            this.toggle_terminal(&ToggleTerminal, w, cx)
                        })),
                    )
                    .child(
                        Self::icon_button(
                            "settings",
                            "settings",
                            format!(
                                "Change accent, now {}",
                                TRAYS[cx.try_global::<OrynTheme>().map(|t| t.tray).unwrap_or(0)
                                    % TRAYS.len()]
                                .name
                            ),
                            false,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| this.cycle_tray(&CycleTray, w, cx))),
                    ),
            )
            .when(caption, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .child(
                            div()
                                .w(px(1.0))
                                .h(px(20.0))
                                .mr(px(6.0))
                                .bg(white_alpha(0.1)),
                        )
                        .child(Self::caption("win-min", "win-min", WindowControlArea::Min))
                        .child(Self::caption("win-max", "win-max", WindowControlArea::Max))
                        .child(Self::caption(
                            "win-close",
                            "win-close",
                            WindowControlArea::Close,
                        )),
                )
            })
    }

    fn caption(id: &'static str, name: &'static str, area: WindowControlArea) -> Stateful<Div> {
        div()
            .id(id)
            .window_control_area(area)
            .flex()
            .items_center()
            .justify_center()
            .w(px(30.0))
            .h(px(28.0))
            .rounded(px(4.0))
            .hover(|s| s.bg(white_alpha(0.08)))
            .child(oryn_icon(name, 12.0, hex(0xa1a1a6)))
    }

    fn tool(
        id: &'static str,
        name: &'static str,
        label: Option<&'static str>,
        aria: &'static str,
        last: bool,
        active: bool,
        p: &Palette,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(aria)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .h(px(28.0))
            .px(px(12.0))
            .text_size(px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if active {
                p.text_strong
            } else {
                p.text_secondary
            })
            .cursor_pointer()
            .when(!last, |d| d.border_r_1().border_color(white_alpha(0.06)))
            .when(active, |d| d.bg(white_alpha(0.14)))
            .when(!active, |d| {
                d.hover(|s| s.bg(white_alpha(0.09)).text_color(p.text_strong))
            })
            .child(oryn_icon(
                name,
                14.0,
                if active {
                    p.text_strong
                } else {
                    p.text_secondary
                },
            ))
            .when_some(label, |d, l| d.child(l))
    }

    fn group(p: &Palette) -> Div {
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(30.0))
            .rounded(px(6.0))
            .bg(p.control)
            .border_1()
            .border_color(p.control_border)
            .overflow_hidden()
            .shadow(vec![BoxShadow {
                color: black_alpha(0.2),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(3.0),
                spread_radius: px(0.0),
                inset: false,
            }])
    }

    fn render_toolbar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let (back, fwd) = {
            let a = self.active_pane().read(cx);
            (a.can_go_back(), a.can_go_forward())
        };
        div()
            .id("toolbar")
            .role(Role::Toolbar)
            .aria_label("Action toolbar")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.0))
            .h(px(46.0))
            .px(px(18.0))
            .child(
                Self::group(p)
                    .child(
                        Self::tool("back", "nav-back", None, "Back", false, false, p)
                            .when(!back, |d| d.opacity(0.45))
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.with_pane(w, cx, |p, cx| p.go_back(cx))
                            })),
                    )
                    .child(
                        Self::tool("forward", "nav-forward", None, "Forward", false, false, p)
                            .when(!fwd, |d| d.opacity(0.45))
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.with_pane(w, cx, |p, cx| p.go_forward(cx))
                            })),
                    )
                    .child(
                        Self::tool("up", "nav-up", None, "Parent folder", true, false, p).on_click(
                            cx.listener(|this, _, w, cx| {
                                this.with_pane(w, cx, |p, cx| p.go_parent(cx))
                            }),
                        ),
                    ),
            )
            .child(
                Self::group(p)
                    .child(Self::tool(
                        "new-file",
                        "new-file",
                        Some("New File"),
                        "New file",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "new-folder",
                        "new-folder",
                        Some("Folder"),
                        "New folder",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "rename",
                        "rename",
                        Some("Rename"),
                        "Rename",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "duplicate",
                        "duplicate",
                        Some("Duplicate"),
                        "Duplicate",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "delete",
                        "delete",
                        Some("Delete"),
                        "Delete",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "preview",
                        "preview",
                        Some("Preview"),
                        "Preview",
                        true,
                        false,
                        p,
                    )),
            )
            .child(
                Self::group(p)
                    .child(Self::tool(
                        "view-list",
                        "view-list",
                        None,
                        "List view",
                        false,
                        true,
                        p,
                    ))
                    .child(Self::tool(
                        "view-grid",
                        "view-grid",
                        None,
                        "Grid view",
                        false,
                        false,
                        p,
                    ))
                    .child(Self::tool(
                        "view-columns",
                        "view-columns",
                        None,
                        "Columns view",
                        true,
                        false,
                        p,
                    )),
            )
            .child(Self::group(p).child(Self::tool(
                "more",
                "more",
                None,
                "More options",
                true,
                false,
                p,
            )))
    }

    fn render_nav_bar(&self, path: &Path, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut acc = PathBuf::new();
        let mut crumbs = Vec::new();
        for comp in path.components() {
            acc.push(comp.as_os_str());
            let label = match comp {
                std::path::Component::RootDir => "Root (/)".to_string(),
                other => other.as_os_str().to_string_lossy().to_string(),
            };
            crumbs.push((label, acc.clone()));
        }
        let last = crumbs.len().saturating_sub(1);
        let children = crumbs
            .into_iter()
            .enumerate()
            .flat_map(|(i, (label, target))| {
                let is_last = i == last;
                let mut out: Vec<AnyElement> = Vec::new();
                if i > 0 {
                    out.push(
                        div()
                            .mx(px(2.0))
                            .text_size(px(12.0))
                            .text_color(p.text_faint)
                            .child("\u{203a}")
                            .into_any_element(),
                    );
                }
                out.push(
                    div()
                        .id(("crumb", i))
                        .role(Role::Link)
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .text_size(px(12.5))
                        .when(is_last, |d| {
                            d.text_color(p.text_strong)
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(!is_last, |d| d.text_color(p.text_crumb))
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .hover(|s| s.bg(white_alpha(0.08)).text_color(p.text_strong))
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
            .gap(px(10.0))
            .px(px(18.0))
            .pb(px(8.0))
            .h(px(38.0))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .h(px(30.0))
                    .px(px(12.0))
                    .rounded_full()
                    .bg(white_alpha(0.04))
                    .border_1()
                    .border_color(p.card_border)
                    .overflow_hidden()
                    .child(div().mr(px(4.0)).child(icon("folder", 13.0, p.text_muted)))
                    .children(children)
                    .child(div().flex_1())
                    .child(
                        div()
                            .px(px(6.0))
                            .child(oryn_icon("rename", 12.0, hex(0x78787c))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .w(px(250.0))
                    .h(px(30.0))
                    .pl(px(12.0))
                    .pr(px(10.0))
                    .rounded_full()
                    .bg(p.control)
                    .border_1()
                    .border_color(p.control_border)
                    .child(oryn_icon("search", 14.0, p.text_muted))
                    .child(
                        div().flex_1().min_w_0().text_size(px(12.5)).child(
                            Input::new(&self.search)
                                .appearance(false)
                                .text_size(px(12.5))
                                .text_color(p.text),
                        ),
                    )
                    .child(oryn_icon("filter", 14.0, p.text_muted)),
            )
    }

    fn render_status(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let pane = self.active_pane().read(cx);
        let (n, bytes) = pane.marked_summary();
        let (total, total_bytes) = pane.totals();
        let left = if n > 0 {
            format!("{n} of {total} selected ({})", fmt_size(Some(bytes)))
        } else {
            format!("{total} items \u{2014} {}", fmt_size(Some(total_bytes)))
        };
        let path = pane.path.display().to_string();
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(28.0))
            .px(px(18.0))
            .border_t_1()
            .border_color(p.divider)
            .bg(p.bg)
            .text_size(px(11.5))
            .text_color(p.text_muted)
            .child(div().flex_1().child(left))
            .child(
                div()
                    .id("status-term")
                    .role(Role::Button)
                    .aria_label("Toggle terminal")
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .px(px(10.0))
                    .py(px(2.0))
                    .rounded(px(12.0))
                    .bg(white_alpha(0.06))
                    .border_1()
                    .border_color(p.control_border)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text_sidebar)
                    .cursor_pointer()
                    .hover(|s| s.bg(white_alpha(0.12)).text_color(p.text_strong))
                    .on_click(
                        cx.listener(|this, _, w, cx| this.toggle_terminal(&ToggleTerminal, w, cx)),
                    )
                    .child(oryn_icon("status-terminal", 12.0, p.text_sidebar))
                    .child("Terminal"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .min_w_0()
                    .text_color(p.text_dim)
                    .child(div().truncate().child(path)),
            )
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
        if self.placeholder_for != title {
            self.placeholder_for = title.clone();
            let ph = format!("Search {title}\u{2026}");
            self.search
                .update(cx, |s, cx| s.set_placeholder(ph, window, cx));
        }
        let terminal = self.show_terminal.then(|| self.terminal.clone()).flatten();

        div()
            .id("workspace")
            .key_context("Workspace")
            .role(Role::Application)
            .aria_label("Oryn")
            .on_action(cx.listener(Self::switch_pane))
            .on_action(cx.listener(Self::toggle_panes))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::toggle_terminal))
            .on_action(cx.listener(Self::focus_search))
            .on_action(cx.listener(Self::cycle_tray))
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .flex()
            .size_full()
            .bg(p.bg)
            .font_family(ui_font())
            .text_color(p.text)
            .when(self.sidebar, |d| {
                d.child(self.render_sidebar(&path, &p, cx))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.render_header(title.clone(), &p, cx))
                    .child(self.render_toolbar(&p, cx))
                    .child(self.render_nav_bar(&path, &p, cx))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .gap(px(12.0))
                            .px(px(16.0))
                            .pb(px(12.0))
                            .child(self.panes[0].clone())
                            .when(self.dual, |d| d.child(self.panes[1].clone())),
                    )
                    .when_some(terminal, |d, t| {
                        d.child(
                            div()
                                .flex_none()
                                .h(px(250.0))
                                .mx(px(16.0))
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
