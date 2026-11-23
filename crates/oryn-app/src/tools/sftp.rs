use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::input::InputState;
use oryn_core::format::fmt_size;
use oryn_core::services::remote::RemoteSession;
use oryn_core::services::remote::ops;
use oryn_core::services::remote::profile::{self, AuthMethod, RemoteProfile};
use oryn_core::vfs::types::FileItem;
use parking_lot::Mutex;

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

struct Form {
    name: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    user: Entity<InputState>,
    secret: Entity<InputState>,
    use_key: bool,
}

pub struct SftpTool {
    profiles: Vec<RemoteProfile>,
    editing: Option<String>,
    prefill: Option<RemoteProfile>,
    form: Form,
    session: Option<Arc<Mutex<RemoteSession>>>,
    connected: Option<RemoteProfile>,
    remote_dir: String,
    items: Vec<FileItem>,
    selected: Option<usize>,
    local_dir: PathBuf,
    uploads: Vec<PathBuf>,
    fingerprint: Option<String>,
    busy: bool,
    note: Option<(String, bool)>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for SftpTool {}

fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

fn parent(dir: &str) -> String {
    match dir.trim_end_matches('/').rsplit_once('/') {
        Some(("", _)) | None => "/".into(),
        Some((p, _)) => p.into(),
    }
}

fn unknown_host(err: &str) -> Option<String> {
    if !err.contains("Unknown SSH host key") {
        return None;
    }
    let start = err.find("SHA256:")? + 7;
    let rest = &err[start..];
    let end = rest.find(')').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

impl SftpTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut f = |ph: &'static str| input(ph, window, cx, |t, _, cx| t.connect_form(cx));
        let form = Form {
            name: f("Name, like Home server"),
            host: f("Host"),
            port: f("Port"),
            user: f("User"),
            secret: f("Password"),
            use_key: false,
        };
        form.port.update(cx, |s, cx| s.set_value("22", window, cx));
        let mut tool = Self {
            profiles: profile::load_profiles().unwrap_or_default(),
            editing: None,
            prefill: None,
            form,
            session: None,
            connected: None,
            remote_dir: "/".into(),
            items: Vec::new(),
            selected: None,
            local_dir: ctx.dir.clone(),
            uploads: ctx
                .targets
                .iter()
                .filter(|p| p.is_file())
                .cloned()
                .collect(),
            fingerprint: None,
            busy: false,
            note: None,
            task: None,
            focus: cx.focus_handle(),
        };
        if let Some(pr) = ctx
            .tag
            .as_ref()
            .and_then(|id| tool.profiles.iter().find(|p| &p.id == id).cloned())
        {
            tool.pick(pr, cx);
        }
        tool
    }

    fn pick(&mut self, pr: RemoteProfile, cx: &mut Context<Self>) {
        if pr.auth_type == AuthMethod::Password && pr.password.is_none() {
            self.editing = Some(pr.id.clone());
            self.note = Some((
                format!("Type the password for {}, then press Connect.", pr.name),
                false,
            ));
            self.prefill = Some(pr);
            cx.notify();
        } else {
            self.connect(pr, cx);
        }
    }

    fn form_profile(&self, cx: &App) -> Option<RemoteProfile> {
        let host = value(&self.form.host, cx).trim().to_string();
        let user = value(&self.form.user, cx).trim().to_string();
        if host.is_empty() || user.is_empty() {
            return None;
        }
        let secret = value(&self.form.secret, cx);
        let name = value(&self.form.name, cx).trim().to_string();
        Some(RemoteProfile {
            id: self.editing.clone().unwrap_or_default(),
            name: if name.is_empty() { host.clone() } else { name },
            port: value(&self.form.port, cx).trim().parse().unwrap_or(22),
            username: user,
            auth_type: if self.form.use_key {
                AuthMethod::Key
            } else {
                AuthMethod::Password
            },
            password: (!self.form.use_key && !secret.is_empty()).then(|| secret.clone()),
            key_path: (self.form.use_key && !secret.is_empty()).then_some(secret),
            host,
            ..RemoteProfile::default()
        })
    }

    fn save_profile(&mut self, mut prof: RemoteProfile) -> RemoteProfile {
        let password = prof.password.take();
        if let Some(existing) = self.profiles.iter().find(|p| p.id == prof.id) {
            prof.expected_fingerprint = prof
                .expected_fingerprint
                .or(existing.expected_fingerprint.clone());
        }
        if let Ok(list) = profile::save_profile(prof.clone()) {
            if prof.id.is_empty()
                && let Some(saved) = list
                    .iter()
                    .rev()
                    .find(|p| p.host == prof.host && p.username == prof.username)
            {
                prof.id = saved.id.clone();
            }
            self.profiles = list;
        }
        prof.password = password;
        prof
    }

    fn connect_form(&mut self, cx: &mut Context<Self>) {
        let Some(prof) = self.form_profile(cx) else {
            self.note = Some(("Fill in the host and user.".into(), true));
            cx.notify();
            return;
        };
        let prof = self.save_profile(prof);
        self.connect(prof, cx);
    }

    fn connect(&mut self, prof: RemoteProfile, cx: &mut Context<Self>) {
        self.busy = true;
        self.note = Some((format!("Connecting to {}\u{2026}", prof.host), false));
        self.fingerprint = None;
        let attempt = prof.clone();
        self.task = Some(run_bg(
            cx,
            move || RemoteSession::connect(&attempt).map_err(|e| format!("{e:#}")),
            move |t, res, cx| {
                t.busy = false;
                match res {
                    Ok(session) => {
                        t.session = Some(Arc::new(Mutex::new(session)));
                        t.remote_dir = prof.initial_path.clone().unwrap_or_else(|| "/".into());
                        t.note = None;
                        t.connected = Some(prof);
                        t.list(cx);
                    }
                    Err(e) => {
                        t.fingerprint = unknown_host(&e);
                        t.connected = Some(prof);
                        t.note = Some((e, true));
                    }
                }
            },
        ));
        cx.notify();
    }

    fn trust(&mut self, cx: &mut Context<Self>) {
        let (Some(fp), Some(mut prof)) = (self.fingerprint.take(), self.connected.clone()) else {
            return;
        };
        prof.expected_fingerprint = Some(format!("SHA256:{fp}"));
        let prof = self.save_profile(prof);
        self.connect(prof, cx);
    }

    fn disconnect(&mut self, cx: &mut Context<Self>) {
        self.session = None;
        self.connected = None;
        self.items.clear();
        self.note = None;
        cx.notify();
    }

    fn list(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let dir = self.remote_dir.clone();
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || ops::list_dir(&session.lock(), &dir).map_err(|e| format!("{e:#}")),
            |t, res, _| {
                t.busy = false;
                t.selected = None;
                match res {
                    Ok(l) => t.items = l.items.into_iter().filter(|i| i.base != "..").collect(),
                    Err(e) => t.note = Some((e, true)),
                }
            },
        ));
        cx.notify();
    }

    fn open_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.remote_dir = dir;
        self.list(cx);
    }

    fn download(&mut self, cx: &mut Context<Self>) {
        let (Some(session), Some(item)) = (
            self.session.clone(),
            self.selected.and_then(|i| self.items.get(i)).cloned(),
        ) else {
            return;
        };
        if item.is_dir {
            self.note = Some(("Pick a file to download.".into(), true));
            cx.notify();
            return;
        }
        let remote = join(&self.remote_dir, &item.base);
        let local = self.local_dir.join(&item.base);
        self.busy = true;
        self.note = Some((format!("Downloading {}\u{2026}", item.base), false));
        self.task = Some(run_bg(
            cx,
            move || {
                ops::download_file(&session.lock(), &remote, &local).map_err(|e| format!("{e:#}"))
            },
            move |t, res, cx| {
                t.busy = false;
                t.note = Some(match res {
                    Ok(n) => {
                        cx.emit(ToolEvent::Changed);
                        (
                            format!(
                                "Downloaded {} ({}) to {}.",
                                item.base,
                                fmt_size(Some(n)),
                                t.local_dir.display()
                            ),
                            false,
                        )
                    }
                    Err(e) => (e, true),
                });
            },
        ));
        cx.notify();
    }

    fn upload(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.clone() else {
            return;
        };
        if self.uploads.is_empty() {
            self.note = Some((
                "Select files in the pane first, then open SFTP.".into(),
                true,
            ));
            cx.notify();
            return;
        }
        let files = self.uploads.clone();
        let dir = self.remote_dir.clone();
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || {
                let s = session.lock();
                let mut failed = Vec::new();
                for f in &files {
                    let name = f
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if let Err(e) = ops::upload_file(&s, f, &join(&dir, &name)) {
                        failed.push(format!("{name}: {e:#}"));
                    }
                }
                (files.len(), failed)
            },
            |t, (n, failed), cx| {
                t.busy = false;
                t.note = Some(if failed.is_empty() {
                    (
                        format!("Uploaded {n} file{}.", if n == 1 { "" } else { "s" }),
                        false,
                    )
                } else {
                    (failed.join("; "), true)
                });
                t.list(cx);
            },
        ));
        cx.notify();
    }

    fn render_login(&mut self, p: &Palette, cx: &mut Context<Self>) -> Div {
        let saved = self.profiles.iter().enumerate().map(|(i, prof)| {
            let pr = prof.clone();
            let id = prof.id.clone();
            div()
                .id(("prof", i))
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(12.0))
                .h(px(44.0))
                .border_b_1()
                .border_color(white_alpha(0.06))
                .cursor_pointer()
                .hover(|s| s.bg(white_alpha(0.04)))
                .on_click(cx.listener(move |this, _, _, cx| this.pick(pr.clone(), cx)))
                .child(icon("server", 16.0, p.accent))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.text_strong)
                                .child(prof.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(p.text_muted)
                                .child(format!("{}@{}:{}", prof.username, prof.host, prof.port)),
                        ),
                )
                .child(
                    div()
                        .id(("prof-del", i))
                        .text_size(px(11.0))
                        .text_color(p.text_muted)
                        .hover(|s| s.text_color(hex(0xff6961)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if let Ok(list) = profile::delete_profile(&id) {
                                this.profiles = list;
                            }
                            cx.notify();
                        }))
                        .child("Remove"),
                )
        });
        let has_saved = !self.profiles.is_empty();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .gap(px(12.0))
            .child(
                list_box(p)
                    .w(px(320.0))
                    .flex_none()
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_muted)
                            .child("SAVED CONNECTIONS"),
                    )
                    .when(!has_saved, |d| {
                        d.child(empty("server", "No saved connections yet.", p))
                    })
                    .child(
                        div()
                            .id("profiles")
                            .flex_1()
                            .overflow_y_scroll()
                            .children(saved),
                    ),
            )
            .child(
                list_box(p)
                    .p(px(16.0))
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_strong)
                            .child("Connect to a server"),
                    )
                    .child(div().flex().child(field(&self.form.name, p)))
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(field(&self.form.host, p))
                            .child(div().w(px(80.0)).flex().child(field(&self.form.port, p))),
                    )
                    .child(div().flex().child(field(&self.form.user, p)))
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                chip("auth-pass", "Password", !self.form.use_key, p).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.form.use_key = false;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(
                                chip("auth-key", "Private key file", self.form.use_key, p)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.form.use_key = true;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(div().flex().child(field(&self.form.secret, p)))
                    .child(
                        div().flex().justify_end().child(
                            button(
                                "sftp-connect",
                                if self.busy {
                                    "Connecting\u{2026}"
                                } else {
                                    "Connect"
                                },
                                Tone::Primary,
                                p,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.connect_form(cx))),
                        ),
                    )
                    .child(status_line(
                        "Passwords are used for this session only and never saved.",
                        false,
                        p,
                    )),
            )
    }

    fn render_browser(&self, p: &Palette, cx: &mut Context<Self>) -> Div {
        let rows = self.items.iter().enumerate().map(|(i, item)| {
            let on = self.selected == Some(i);
            let name = item.base.clone();
            let is_dir = item.is_dir;
            let target = join(&self.remote_dir, &item.base);
            div()
                .id(("remote", i))
                .h(px(30.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .cursor_pointer()
                .text_size(px(12.5))
                .when(on, |d| d.bg(with_alpha(p.accent, 0.22)))
                .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.04))))
                .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                    this.selected = Some(i);
                    if ev.click_count() > 1 {
                        if is_dir {
                            this.open_dir(target.clone(), cx);
                        } else {
                            this.download(cx);
                        }
                    }
                    cx.notify();
                }))
                .child(file_icon(&name, is_dir, false, p))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(p.text_strong)
                        .child(name),
                )
                .child(
                    div()
                        .w(px(150.0))
                        .text_size(px(11.0))
                        .text_color(p.text_muted)
                        .child(item.mtime.clone()),
                )
                .child(
                    div()
                        .w(px(70.0))
                        .text_right()
                        .text_size(px(11.0))
                        .text_color(p.text_muted)
                        .child(if is_dir {
                            "Folder".into()
                        } else {
                            fmt_size(item.size)
                        }),
                )
        });
        let up = parent(&self.remote_dir);
        let at_root = self.remote_dir == "/";
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        button("sftp-up", "\u{2191}", Tone::Plain, p)
                            .when(at_root, |d| d.opacity(0.4))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.open_dir(up.clone(), cx)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(p.text_strong)
                            .child(self.remote_dir.clone()),
                    )
                    .child(
                        button("sftp-refresh", "Refresh", Tone::Plain, p)
                            .on_click(cx.listener(|this, _, _, cx| this.list(cx))),
                    )
                    .child(
                        button("sftp-down", "Download", Tone::Plain, p)
                            .when(self.selected.is_none(), |d| d.opacity(0.5))
                            .on_click(cx.listener(|this, _, _, cx| this.download(cx))),
                    )
                    .child(
                        button(
                            "sftp-upload",
                            if self.uploads.is_empty() {
                                "Upload".to_string()
                            } else {
                                format!("Upload {} selected", self.uploads.len())
                            },
                            Tone::Primary,
                            p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.upload(cx))),
                    ),
            )
            .child(if self.items.is_empty() {
                list_box(p).child(empty(
                    "folder",
                    if self.busy {
                        "Loading\u{2026}"
                    } else {
                        "This folder is empty."
                    },
                    p,
                ))
            } else {
                list_box(p).child(
                    div()
                        .id("remote-rows")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(rows),
                )
            })
            .child(status_line(
                format!("Downloads go to {}", self.local_dir.display()),
                false,
                p,
            ))
    }
}

impl Focusable for SftpTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SftpTool {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        if let Some(pr) = self.prefill.take() {
            self.form
                .name
                .update(cx, |s, cx| s.set_value(pr.name.clone(), window, cx));
            self.form
                .host
                .update(cx, |s, cx| s.set_value(pr.host.clone(), window, cx));
            self.form
                .port
                .update(cx, |s, cx| s.set_value(pr.port.to_string(), window, cx));
            self.form
                .user
                .update(cx, |s, cx| s.set_value(pr.username.clone(), window, cx));
            self.form.use_key = pr.auth_type == AuthMethod::Key;
            let secret = pr.key_path.clone().unwrap_or_default();
            self.form.secret.update(cx, |s, cx| {
                s.set_value(secret, window, cx);
                s.focus(window, cx);
            });
        }
        let subtitle = match (&self.connected, &self.session) {
            (Some(c), Some(_)) => format!("Connected to {}@{}", c.username, c.host),
            _ => "Browse a server over SSH, and move files both ways.".into(),
        };
        let body = if self.session.is_some() {
            self.render_browser(&p, cx)
        } else {
            self.render_login(&p, cx)
        };
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().flex_1().min_w_0().child(header("SFTP", subtitle, &p)))
                    .when(self.session.is_some(), |d| {
                        d.child(button("sftp-disconnect", "Disconnect", Tone::Plain, &p).on_click(cx.listener(|this, _, _, cx| this.disconnect(cx))))
                    }),
            )
            .when_some(self.fingerprint.clone(), |d, fp| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(with_alpha(hex(0xff9f0a), 0.12))
                        .border_1()
                        .border_color(with_alpha(hex(0xff9f0a), 0.4))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.0))
                                .text_color(p.text_strong)
                                .child(format!("First connection to this server. Check that its key fingerprint is SHA256:{fp} before you trust it.")),
                        )
                        .child(button("sftp-trust", "Trust and connect", Tone::Primary, &p).on_click(cx.listener(|this, _, _, cx| this.trust(cx)))),
                )
            })
            .child(body)
            .when(self.fingerprint.is_none(), |d| d.children(self.note.clone().map(|(s, e)| status_line(s, e, &p))))
    }
}

#[cfg(test)]
mod tests {
    use super::{join, parent, unknown_host};

    #[test]
    fn remote_paths_and_host_prompt() {
        assert_eq!(join("/", "a"), "/a");
        assert_eq!(join("/srv", "a"), "/srv/a");
        assert_eq!(parent("/srv/www"), "/srv");
        assert_eq!(parent("/srv"), "/");
        assert_eq!(parent("/"), "/");
        let err = "Unknown SSH host key for h:22 (Fingerprint: SHA256:abc+/9). To connect, verify";
        assert_eq!(unknown_host(err).as_deref(), Some("abc+/9"));
        assert!(unknown_host("SSH password authentication failed").is_none());
    }
}
