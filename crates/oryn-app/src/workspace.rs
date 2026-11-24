use std::path::{Path, PathBuf};

use crate::tools::ToolKind;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::fmt_size;
use oryn_core::settings::Settings;

use crate::actions::*;
use crate::icons::{icon, oryn_icon};
use crate::pane::{FilePane, ViewMode};
use crate::terminal::TerminalView;
use crate::theme::{
    OrynTheme, Palette, TAGS, TRAYS, black_alpha, hex, palette, ui_font, white_alpha, with_alpha,
};

mod batch_rename;
mod dialog;
mod extras;
mod jobs;
mod menu;
mod palette;
mod quick;
mod settings_view;
mod title_bar;
mod tool_sheet;
mod updates;
mod watch;

use dialog::Dialog;
use jobs::Job;

const TRAFFIC_LIGHTS: f32 = 78.0;

pub struct Workspace {
    favorites: oryn_core::favorites::Favorites,
    profiles: Vec<oryn_core::services::remote::profile::RemoteProfile>,
    clipboard: Option<(oryn_core::batch::BatchKind, Vec<PathBuf>)>,
    info_task: Option<Task<()>>,
    tabs: [Vec<Entity<FilePane>>; 2],
    tab_ix: [usize; 2],
    view: ViewMode,
    active: usize,
    dual: bool,
    sidebar: bool,
    terminal: Option<Entity<TerminalView>>,
    show_terminal: bool,
    search: Entity<InputState>,
    dialog_input: Entity<InputState>,
    dialog: Option<Dialog>,
    dialog_error: Option<SharedString>,
    dialog_focus: FocusHandle,
    job: Option<Job>,
    menu: Option<menu::Menu>,
    batch: Option<batch_rename::BatchRename>,
    palette: Option<palette::CommandPalette>,
    settings: Settings,
    settings_open: Option<FocusHandle>,
    tool: Option<tool_sheet::ToolHost>,
    quick: Option<(Entity<crate::quick_view::QuickView>, Subscription)>,
    placeholder_for: SharedString,
    update: Option<oryn_core::update::Release>,
    installing: bool,
    updates: std::sync::Arc<dyn crate::update::UpdateSource>,
    _update_task: Option<Task<()>>,
    watch: Option<watch::Watch>,
    chrome: std::rc::Rc<dyn crate::chrome::WindowChrome>,
    drag_armed: bool,
}

struct Place {
    label: &'static str,
    icon: &'static str,
    path: PathBuf,
}

pub(crate) fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

type Hook = fn(&mut Workspace, &mut Window, &mut Context<Workspace>);

fn fav_look(path: &Path, home: &Path) -> (String, &'static str) {
    if path == home {
        return ("Home".into(), "house");
    }
    let label = oryn_core::favorites::Favorites::label(path);
    let icon = if path.parent() == Some(home) {
        match label.as_str() {
            "Desktop" => "monitor",
            "Documents" => "file-text",
            "Downloads" => "download",
            _ => "folder",
        }
    } else {
        "folder"
    };
    (label, icon)
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

impl Workspace {
    pub fn new(left: PathBuf, right: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        let view = match settings.view.as_str() {
            "grid" => ViewMode::Grid,
            "columns" => ViewMode::Columns,
            _ => ViewMode::List,
        };
        let l = Self::spawn_pane(left, settings.dual, window, cx);
        let r = Self::spawn_pane(right, settings.dual, window, cx);
        for pane in [&l, &r] {
            pane.update(cx, |p, cx| {
                p.set_view(view);
                p.set_folders_on_top(settings.folders_on_top, cx);
                if !settings.show_hidden {
                    p.show_hidden = false;
                    p.refresh(None, cx);
                }
            });
        }
        let search = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe(&search, |this, state, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                let pane = this.active_pane().clone();
                pane.update(cx, |p, cx| p.set_filter(&text, cx));
            }
        })
        .detach();
        let dialog_input = cx.new(|cx| InputState::new(window, cx));
        cx.subscribe_in(
            &dialog_input,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.confirm_prompt(&ConfirmPrompt, window, cx);
                }
            },
        )
        .detach();
        let focus = l.read(cx).focus.clone();
        window.focus(&focus, cx);
        let check = settings.check_updates;
        let mut this = Self {
            favorites: oryn_core::favorites::Favorites::load(&home_dir()),
            profiles: oryn_core::services::remote::profile::load_profiles().unwrap_or_default(),
            clipboard: None,
            info_task: None,
            tabs: [vec![l], vec![r]],
            tab_ix: [0, 0],
            view,
            active: 0,
            dual: settings.dual,
            sidebar: settings.sidebar,
            terminal: None,
            show_terminal: false,
            search,
            dialog_input,
            dialog: None,
            dialog_error: None,
            dialog_focus: cx.focus_handle(),
            job: None,
            menu: None,
            batch: None,
            palette: None,
            settings,
            settings_open: None,
            tool: None,
            quick: None,
            placeholder_for: SharedString::default(),
            update: None,
            installing: false,
            updates: std::sync::Arc::new(crate::update::GitHubUpdates::default()),
            _update_task: None,
            watch: None,
            chrome: std::rc::Rc::new(crate::chrome::NativeChrome::for_platform()),
            drag_armed: false,
        };
        if std::env::var_os("ORYN_NO_WATCH").is_none() {
            this.watch = Some(Self::start_watch(cx));
        }
        if check {
            this.check_updates(cx);
        }
        this
    }

    #[cfg(test)]
    pub fn quick_view(&self) -> Option<Entity<crate::quick_view::QuickView>> {
        self.quick.as_ref().map(|(v, _)| v.clone())
    }

    #[cfg(test)]
    pub fn open_overlays(&self) -> Vec<&'static str> {
        [
            ("settings", self.settings_open.is_some()),
            ("palette", self.palette.is_some()),
            ("tool", self.tool.is_some()),
            ("quick", self.quick.is_some()),
            ("menu", self.menu.is_some()),
            ("terminal", self.show_terminal),
            ("dialog", self.dialog.is_some()),
        ]
        .into_iter()
        .filter(|(_, on)| *on)
        .map(|(n, _)| n)
        .collect()
    }

    fn spawn_pane(
        path: PathBuf,
        dual: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<FilePane> {
        let pane = cx.new(|cx| {
            let mut pane = FilePane::new(path, cx);
            pane.dual = dual;
            pane
        });
        cx.observe(&pane, |this, _, cx| {
            this.remember_paths(cx);
            cx.notify()
        })
        .detach();
        cx.subscribe_in(&pane, window, Self::on_pane_event).detach();
        pane
    }

    pub fn left(&self) -> Entity<FilePane> {
        self.pane(0).clone()
    }

    fn pane(&self, side: usize) -> &Entity<FilePane> {
        &self.tabs[side][self.tab_ix[side]]
    }

    fn active_pane(&self) -> &Entity<FilePane> {
        self.pane(self.active)
    }

    fn new_tab(&mut self, _: &NewTab, window: &mut Window, cx: &mut Context<Self>) {
        let side = self.active;
        let path = self.active_pane().read(cx).path.clone();
        let pane = Self::spawn_pane(path, self.dual, window, cx);
        let view = self.view;
        let hidden = self.settings.show_hidden;
        let folders = self.settings.folders_on_top;
        pane.update(cx, |p, cx| {
            p.set_view(view);
            p.set_folders_on_top(folders, cx);
            if !hidden {
                p.show_hidden = false;
                p.refresh(None, cx);
            }
        });
        self.tabs[side].push(pane);
        self.tab_ix[side] = self.tabs[side].len() - 1;
        self.focus_pane(side, window, cx);
    }

    pub(super) fn set_view(&mut self, view: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        self.view = view;
        for pane in self.tabs.iter().flatten().cloned().collect::<Vec<_>>() {
            pane.update(cx, |p, cx| {
                p.set_view(view);
                cx.notify();
            });
        }
        self.persist(cx);
        self.focus_pane(self.active, window, cx);
    }

    fn close_tab_at(
        &mut self,
        side: usize,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs[side].len() <= 1 || ix >= self.tabs[side].len() {
            return;
        }
        self.tabs[side].remove(ix);
        let current = self.tab_ix[side];
        if current > ix || current >= self.tabs[side].len() {
            self.tab_ix[side] = current.saturating_sub(1);
        }
        self.focus_pane(side, window, cx);
    }

    fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let side = self.active;
        self.close_tab_at(side, self.tab_ix[side], window, cx);
    }

    fn select_tab(&mut self, side: usize, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix < self.tabs[side].len() {
            self.tab_ix[side] = ix;
            self.focus_pane(side, window, cx);
        }
    }

    fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        let side = self.active;
        let n = self.tabs[side].len();
        self.select_tab(side, (self.tab_ix[side] + 1) % n, window, cx);
    }

    fn prev_tab(&mut self, _: &PrevTab, window: &mut Window, cx: &mut Context<Self>) {
        let side = self.active;
        let n = self.tabs[side].len();
        self.select_tab(side, (self.tab_ix[side] + n - 1) % n, window, cx);
    }

    fn focus_pane(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active = ix;
        let f = self.pane(ix).read(cx).focus.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    fn switch_pane(&mut self, _: &SwitchPane, window: &mut Window, cx: &mut Context<Self>) {
        if self.dual {
            self.focus_pane(1 - self.active, window, cx);
        }
    }

    pub(super) fn toggle_panes(
        &mut self,
        _: &TogglePanes,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dual = !self.dual;
        let dual = self.dual;
        for pane in self.tabs.iter().flatten().cloned().collect::<Vec<_>>() {
            pane.update(cx, |p, _| p.dual = dual);
        }
        if !self.dual {
            self.focus_pane(0, window, cx);
        }
        self.persist(cx);
        cx.notify();
    }

    pub(super) fn toggle_sidebar(
        &mut self,
        _: &ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar = !self.sidebar;
        self.persist(cx);
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
        self.show_terminal_at(cwd, window, cx);
    }

    pub(super) fn show_terminal_at(
        &mut self,
        cwd: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .terminal
            .as_ref()
            .is_some_and(|t| t.read(cx).has_exited())
        {
            self.terminal = None;
        }
        let term = self
            .terminal
            .get_or_insert_with(|| cx.new(|cx| TerminalView::new(&cwd, cx)))
            .clone();
        term.update(cx, |t, _| t.follow(&cwd));
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
        self.persist(cx);
    }

    pub(super) fn persist(&mut self, cx: &mut Context<Self>) {
        let s = &mut self.settings;
        s.view = match self.view {
            ViewMode::List => "list",
            ViewMode::Grid => "grid",
            ViewMode::Columns => "columns",
        }
        .into();
        s.dual = self.dual;
        s.sidebar = self.sidebar;
        s.tray = cx.try_global::<OrynTheme>().map(|t| t.tray).unwrap_or(0);
        s.save();
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

    fn section_header(
        &self,
        key: &'static str,
        title: &'static str,
        plus: Option<(&'static str, Hook)>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let collapsed = self.is_collapsed(key);
        let chevron = oryn_icon("section-chevron", 10.0, p.text_muted).when(collapsed, |s| {
            s.with_transformation(Transformation::rotate(percentage(0.75)))
        });
        div()
            .id(SharedString::from(format!("section-{key}")))
            .debug_selector(|| format!("section-{key}"))
            .role(Role::Button)
            .aria_label(format!(
                "{title}, {}",
                if collapsed { "collapsed" } else { "expanded" }
            ))
            .flex()
            .items_center()
            .h(px(28.0))
            .pl(px(8.0))
            .pr(px(6.0))
            .mt(px(10.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|s| s.bg(white_alpha(0.03)))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_section(key, cx)))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap(px(5.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_muted)
                    .child(chevron)
                    .child(title),
            )
            .when_some(plus, |d, (label, hook)| {
                d.child(
                    div()
                        .id(SharedString::from(format!("{key}-add")))
                        .debug_selector(|| format!("{key}-add"))
                        .role(Role::Button)
                        .aria_label(label)
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(20.0))
                        .rounded(px(4.0))
                        .text_size(px(14.0))
                        .text_color(p.text_muted)
                        .hover(|s| s.bg(white_alpha(0.08)).text_color(p.text_strong))
                        .on_click(cx.listener(move |this, _, w, cx| {
                            cx.stop_propagation();
                            hook(this, w, cx)
                        }))
                        .child("+"),
                )
            })
    }

    fn sidebar_item(
        id: impl Into<ElementId>,
        label: SharedString,
        lead: AnyElement,
        selected: bool,
        p: &Palette,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(Role::Link)
            .aria_label(label.clone())
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
            .child(div().flex_1().min_w_0().truncate().child(label))
    }

    fn render_sidebar(
        &self,
        current: &Path,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let home = home_dir();
        let favs: Vec<AnyElement> = self
            .favorites
            .paths
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let (label, icon_name) = fav_look(path, &home);
                let target = path.clone();
                let gone = path.clone();
                let group = SharedString::from(format!("fav-group-{i}"));
                Self::sidebar_item(
                    ("fav", i),
                    label.into(),
                    icon(icon_name, 16.0, p.text_sidebar).into_any_element(),
                    current == path.as_path(),
                    p,
                )
                .debug_selector(|| format!("fav-{i}"))
                .group(group.clone())
                .on_click(
                    cx.listener(move |this, _, window, cx| this.go_to(target.clone(), window, cx)),
                )
                .child(
                    div()
                        .id(("fav-remove", i))
                        .debug_selector(|| format!("fav-remove-{i}"))
                        .role(Role::Button)
                        .aria_label("Remove from favourites")
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(18.0))
                        .rounded(px(4.0))
                        .text_size(px(13.0))
                        .text_color(p.text_muted)
                        .opacity(0.0)
                        .group_hover(group, |s| s.opacity(1.0))
                        .hover(|s| s.bg(white_alpha(0.1)).text_color(p.text_strong))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.remove_favorite(&gone, cx);
                        }))
                        .child("\u{00d7}"),
                )
                .into_any_element()
            })
            .collect();
        let drive = system_drive();
        let drive_target = drive.path.clone();
        let drive_item = Self::sidebar_item(
            ("drive", 0usize),
            drive.label.into(),
            icon(drive.icon, 16.0, p.accent).into_any_element(),
            current == drive.path,
            p,
        )
        .on_click(
            cx.listener(move |this, _, window, cx| this.go_to(drive_target.clone(), window, cx)),
        );
        let servers: Vec<AnyElement> = self
            .profiles
            .iter()
            .enumerate()
            .map(|(i, prof)| {
                let id = prof.id.clone();
                Self::sidebar_item(
                    ("server", i),
                    prof.name.clone().into(),
                    icon("server", 16.0, p.text_sidebar).into_any_element(),
                    false,
                    p,
                )
                .debug_selector(|| format!("server-{i}"))
                .on_click(cx.listener(move |this, _, w, cx| {
                    this.open_tool_with(ToolKind::Sftp, Some(id.clone()), w, cx)
                }))
                .into_any_element()
            })
            .collect();
        let tags: Vec<AnyElement> = TAGS
            .iter()
            .enumerate()
            .map(|(i, (label, color))| {
                Self::sidebar_item(
                    ("tag", i),
                    (*label).into(),
                    div()
                        .size(px(9.0))
                        .rounded_full()
                        .bg(hex(*color))
                        .mx(px(3.5))
                        .into_any_element(),
                    false,
                    p,
                )
                .debug_selector(|| format!("tag-{label}"))
                .on_click(cx.listener(move |this, _, w, cx| this.click_tag(label, w, cx)))
                .into_any_element()
            })
            .collect();
        let no_servers = servers.is_empty();

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
                self.drag_region(div().id("sidebar-top"), cx)
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
                            .debug_selector(|| "sidebar-toggle".into())
                            .occlude()
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
                    .child(self.section_header(
                        "favorites",
                        "FAVORITES",
                        Some(("Add this folder to favourites", |this, w, cx| {
                            this.add_current_favorite(w, cx)
                        })),
                        p,
                        cx,
                    ))
                    .when(!self.is_collapsed("favorites"), |d| {
                        d.children(favs).when(self.favorites.paths.is_empty(), |d| {
                            d.child(Self::sidebar_hint("Press + to add this folder", p))
                        })
                    })
                    .child(self.section_header("locations", "LOCATIONS", None, p, cx))
                    .when(!self.is_collapsed("locations"), |d| d.child(drive_item))
                    .child(self.section_header(
                        "remote",
                        "REMOTE (SFTP)",
                        Some(("Add a server", |this, w, cx| {
                            this.open_tool(ToolKind::Sftp, w, cx)
                        })),
                        p,
                        cx,
                    ))
                    .when(!self.is_collapsed("remote"), |d| {
                        d.children(servers).when(no_servers, |d| {
                            d.child(Self::sidebar_hint("No saved servers", p))
                        })
                    })
                    .child(self.section_header(
                        "tags",
                        "TAGS",
                        Some(("Tag the selection", |this, w, cx| {
                            this.open_tool(ToolKind::Tags, w, cx)
                        })),
                        p,
                        cx,
                    ))
                    .when(!self.is_collapsed("tags"), |d| d.children(tags)),
            )
    }

    fn sidebar_hint(text: &'static str, p: &Palette) -> Div {
        div()
            .px(px(12.0))
            .py(px(6.0))
            .text_size(px(11.0))
            .text_color(p.text_muted)
            .child(text)
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
            .debug_selector(|| format!("btn-{id}"))
            .occlude()
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
        let caption = self.chrome.shows_captions();
        self.drag_region(div().id("header"), cx)
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
                                .debug_selector(|| "sidebar-show".into())
                                .occlude()
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
                    .debug_selector(|| "folder-title".into())
                    .role(Role::Heading)
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(p.text_strong)
                    .child(title.clone()),
            )
            .child(self.render_tabs(p, cx))
            .when_some(self.update.clone(), |d, rel| {
                d.child(
                    div()
                        .id("update-pill")
                        .debug_selector(|| "update-pill".into())
                        .occlude()
                        .role(Role::Button)
                        .aria_label(format!("Update to Oryn {}", rel.version))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .h(px(26.0))
                        .px(px(10.0))
                        .rounded(px(13.0))
                        .bg(with_alpha(p.accent, 0.18))
                        .border_1()
                        .border_color(with_alpha(p.accent, 0.5))
                        .text_size(px(12.0))
                        .text_color(p.text_strong)
                        .cursor_pointer()
                        .hover(|s| s.bg(with_alpha(p.accent, 0.28)))
                        .on_click(cx.listener(|this, _, w, cx| this.offer_update(w, cx)))
                        .child(icon("download", 13.0, p.accent))
                        .child(format!("Update {}", rel.version)),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        Self::icon_button("add-tab", "add-tab", "New tab", false, p)
                            .on_click(cx.listener(|this, _, w, cx| this.new_tab(&NewTab, w, cx))),
                    )
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
                    .child(
                        Self::icon_button("remote", "remote", "Remote servers", false, p).on_click(
                            cx.listener(|this, _, w, cx| this.open_tool(ToolKind::Sftp, w, cx)),
                        ),
                    )
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
                        Self::icon_button("settings", "settings", "Settings", false, p).on_click(
                            cx.listener(|this, _, w, cx| this.open_settings(&OpenSettings, w, cx)),
                        ),
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
                        .child(self.caption("win-min", WindowControlArea::Min, cx))
                        .child(self.caption("win-max", WindowControlArea::Max, cx))
                        .child(self.caption("win-close", WindowControlArea::Close, cx)),
                )
            })
    }

    fn render_tabs(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let side = self.active;
        let many = self.tabs[side].len() > 1;
        let tabs = self.tabs[side].iter().enumerate().map(|(ix, pane)| {
            let active = ix == self.tab_ix[side];
            let pane = pane.read(cx);
            let name = pane.title();
            let full = pane.path.display().to_string();
            let fg = if active { p.on_accent } else { hex(0xa0a0a5) };
            div()
                .id(("tab", ix))
                .debug_selector(move || format!("tab-{ix}"))
                .occlude()
                .role(Role::Tab)
                .aria_label(SharedString::from(full))
                .aria_selected(active)
                .flex()
                .items_center()
                .gap(px(7.0))
                .max_w(px(200.0))
                .px(px(12.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .border_1()
                .text_size(px(12.5))
                .text_color(fg)
                .cursor_pointer()
                .when(active, |d| {
                    d.bg(p.accent)
                        .border_color(p.accent)
                        .font_weight(FontWeight::SEMIBOLD)
                        .shadow(vec![BoxShadow {
                            color: with_alpha(p.accent, 0.35),
                            offset: point(px(0.0), px(2.0)),
                            blur_radius: px(8.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                })
                .when(!active, |d| {
                    d.bg(white_alpha(0.05))
                        .border_color(p.control_border)
                        .font_weight(FontWeight::MEDIUM)
                        .hover(|s| s.bg(white_alpha(0.1)).text_color(p.text_strong))
                })
                .on_click(cx.listener(move |this, _, w, cx| this.select_tab(side, ix, w, cx)))
                .child(icon("folder", 13.0, fg))
                .child(div().min_w_0().truncate().child(name))
                .when(many, |d| {
                    d.child(
                        div()
                            .id(("tab-close", ix))
                            .role(Role::Button)
                            .aria_label("Close tab")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(15.0))
                            .ml(px(2.0))
                            .rounded_full()
                            .text_size(px(13.0))
                            .opacity(0.6)
                            .hover(|s| s.opacity(1.0).bg(black_alpha(0.25)))
                            .on_click(cx.listener(move |this, _, w, cx| {
                                cx.stop_propagation();
                                this.close_tab_at(side, ix, w, cx);
                            }))
                            .child("\u{00d7}"),
                    )
                })
        });
        div()
            .id("tabs")
            .role(Role::TabList)
            .aria_label("Tabs")
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(tabs)
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
            .debug_selector(|| format!("tool-{id}"))
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
                    .child(
                        Self::tool(
                            "new-file",
                            "new-file",
                            Some("New File"),
                            "New file",
                            false,
                            false,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| this.new_file(&NewFile, w, cx))),
                    )
                    .child(
                        Self::tool(
                            "new-folder",
                            "new-folder",
                            Some("Folder"),
                            "New folder",
                            false,
                            false,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| this.new_folder(&NewFolder, w, cx))),
                    )
                    .child(
                        Self::tool(
                            "rename",
                            "rename",
                            Some("Rename"),
                            "Rename",
                            false,
                            false,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.rename_item(&RenameItem, w, cx)),
                        ),
                    )
                    .child(
                        Self::tool(
                            "duplicate",
                            "duplicate",
                            Some("Duplicate"),
                            "Duplicate",
                            false,
                            false,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| {
                            this.duplicate_items(&DuplicateItems, w, cx)
                        })),
                    )
                    .child(
                        Self::tool(
                            "delete",
                            "delete",
                            Some("Delete"),
                            "Delete",
                            false,
                            false,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.delete_items(&DeleteItems, w, cx)),
                        ),
                    )
                    .child(
                        Self::tool(
                            "preview",
                            "preview",
                            Some("Preview"),
                            "Preview",
                            true,
                            false,
                            p,
                        )
                        .on_click(cx.listener(|this, _, w, cx| this.quick_look(&QuickLook, w, cx))),
                    ),
            )
            .child(
                Self::group(p)
                    .child(
                        Self::tool(
                            "view-list",
                            "view-list",
                            None,
                            "List view",
                            false,
                            self.view == ViewMode::List,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.set_view(ViewMode::List, w, cx)),
                        ),
                    )
                    .child(
                        Self::tool(
                            "view-grid",
                            "view-grid",
                            None,
                            "Grid view",
                            false,
                            self.view == ViewMode::Grid,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.set_view(ViewMode::Grid, w, cx)),
                        ),
                    )
                    .child(
                        Self::tool(
                            "view-columns",
                            "view-columns",
                            None,
                            "Columns view",
                            true,
                            self.view == ViewMode::Columns,
                            p,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.set_view(ViewMode::Columns, w, cx)),
                        ),
                    ),
            )
            .child(
                Self::group(p).child(
                    Self::tool(
                        "more",
                        "more",
                        None,
                        "More options",
                        true,
                        self.menu.is_some(),
                        p,
                    )
                    .on_click(cx.listener(|this, ev: &ClickEvent, w, cx| {
                        let at = ev.position();
                        let at = point(at.x - px(200.0), at.y + px(14.0));
                        this.open_more_menu(at, w, cx)
                    })),
                ),
            )
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
                            .id("path-edit")
                            .debug_selector(|| "path-edit".into())
                            .role(Role::Button)
                            .aria_label("Type a path")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(22.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(white_alpha(0.08)))
                            .on_click(
                                cx.listener(|this, _, w, cx| this.go_to_path(&GoToPath, w, cx)),
                            )
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
                    .when(!self.search.read(cx).value().is_empty(), |d| {
                        d.child(
                            div()
                                .id("filter-clear")
                                .debug_selector(|| "filter-clear".into())
                                .role(Role::Button)
                                .aria_label("Clear the filter")
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(18.0))
                                .rounded_full()
                                .text_size(px(13.0))
                                .text_color(p.text_muted)
                                .cursor_pointer()
                                .hover(|s| s.bg(white_alpha(0.1)).text_color(p.text_strong))
                                .on_click(cx.listener(|this, _, w, cx| this.clear_filter(w, cx)))
                                .child("\u{00d7}"),
                        )
                    }),
            )
    }

    fn render_status(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let pane = self.active_pane().read(cx);
        let (n, bytes) = pane.marked_summary();
        let (total, total_bytes) = pane.totals();
        let left = if let Some(notice) = &pane.notice {
            notice.to_string()
        } else if n > 0 {
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
        gpui_kit::component::window_border().child(self.render_root(window, cx))
    }
}

impl Workspace {
    fn render_root(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        for i in 0..2 {
            if self.pane(i).read(cx).focus.contains_focused(window, cx) {
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
            .on_action(
                cx.listener(|this, _: &ViewList, w, cx| this.set_view(ViewMode::List, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ViewGrid, w, cx| this.set_view(ViewMode::Grid, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ViewColumns, w, cx| this.set_view(ViewMode::Columns, w, cx)),
            )
            .on_action(cx.listener(Self::new_tab))
            .on_action(cx.listener(Self::close_tab))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::prev_tab))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::toggle_terminal))
            .on_action(cx.listener(Self::focus_search))
            .on_action(cx.listener(Self::cycle_tray))
            .on_action(cx.listener(|this, _: &SelectByMask, w, cx| this.open_mask(true, w, cx)))
            .on_action(cx.listener(|this, _: &DeselectByMask, w, cx| this.open_mask(false, w, cx)))
            .on_action(cx.listener(Self::rename_item))
            .on_action(cx.listener(Self::copy_to_other))
            .on_action(cx.listener(Self::move_to_other))
            .on_action(cx.listener(Self::duplicate_items))
            .on_action(cx.listener(Self::compress_zip))
            .on_action(cx.listener(Self::extract_here))
            .on_action(cx.listener(Self::new_folder))
            .on_action(cx.listener(Self::new_file))
            .on_action(cx.listener(Self::delete_items))
            .on_action(cx.listener(Self::delete_permanently))
            .on_action(cx.listener(Self::refresh_panes))
            .on_action(cx.listener(Self::copy_path))
            .on_action(cx.listener(Self::open_context_menu))
            .on_action(cx.listener(Self::open_batch_rename))
            .on_action(cx.listener(Self::open_palette))
            .on_action(cx.listener(Self::open_settings))
            .on_action(cx.listener(Self::quick_look))
            .on_action(
                cx.listener(|this, _: &ShowSearch, w, cx| this.open_tool(ToolKind::Search, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ShowReplace, w, cx| {
                    this.open_tool(ToolKind::Replace, w, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ShowGit, w, cx| this.open_tool(ToolKind::Git, w, cx)))
            .on_action(
                cx.listener(|this, _: &ShowCompare, w, cx| {
                    this.open_tool(ToolKind::Compare, w, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ShowDuplicates, w, cx| {
                this.open_tool(ToolKind::Duplicates, w, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowChecksums, w, cx| {
                this.open_tool(ToolKind::Checksum, w, cx)
            }))
            .on_action(
                cx.listener(|this, _: &ShowDiskSpace, w, cx| this.open_tool(ToolKind::Disk, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ShowTags, w, cx| this.open_tool(ToolKind::Tags, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ShowSftp, w, cx| this.open_tool(ToolKind::Sftp, w, cx)),
            )
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .on_action(cx.listener(Self::add_favorite))
            .on_action(cx.listener(Self::toggle_current_favorite))
            .on_action(cx.listener(Self::toggle_hidden))
            .on_action(cx.listener(|this, _: &SortByName, w, cx| {
                this.sort_active(oryn_core::ui::arrange::SortField::Name, w, cx)
            }))
            .on_action(cx.listener(|this, _: &SortByDate, w, cx| {
                this.sort_active(oryn_core::ui::arrange::SortField::Date, w, cx)
            }))
            .on_action(cx.listener(|this, _: &SortBySize, w, cx| {
                this.sort_active(oryn_core::ui::arrange::SortField::Size, w, cx)
            }))
            .on_action(cx.listener(Self::show_properties))
            .on_action(cx.listener(Self::open_in_terminal))
            .on_action(cx.listener(Self::swap_panes))
            .on_action(cx.listener(Self::cut_items))
            .on_action(cx.listener(Self::copy_items))
            .on_action(cx.listener(Self::paste_items))
            .on_action(cx.listener(Self::go_to_path))
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
                            .child(self.pane(0).clone())
                            .when(self.dual, |d| d.child(self.pane(1).clone())),
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
            .children(self.render_job(&p, cx))
            .children(self.render_batch_rename(&p, cx))
            .children(self.render_palette(&p, cx))
            .children(self.render_settings(&p, cx))
            .children(self.render_tool(&p, cx))
            .children(self.render_quick(cx))
            .children(self.render_menu(&p, cx))
            .children(self.render_dialog(&p, cx))
    }
}
