use std::path::PathBuf;

use gpui_kit::component::input::InputState;
use oryn_core::tools::repo::{self, Change, Commit, DiffLine, LineKind, Status};

use super::*;
use crate::theme::{mono_font, palette};

pub struct GitTool {
    dir: PathBuf,
    root: Option<PathBuf>,
    status: Status,
    selected: Option<String>,
    diff: Vec<DiffLine>,
    history: Option<Vec<Commit>>,
    branches: Option<Vec<String>>,
    message: Entity<InputState>,
    branch_name: Entity<InputState>,
    note: Option<(String, bool)>,
    busy: bool,
    clear_on_success: Option<String>,
    clear_message: Option<String>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for GitTool {}

fn kind_color(c: &Change) -> u32 {
    let ch = if c.is_staged() { c.staged } else { c.unstaged };
    match ch {
        'A' | '?' => 0x34c759,
        'D' => 0xff453a,
        'R' | 'C' => 0xbf5af2,
        'U' => 0xff9f0a,
        _ => 0x0a84ff,
    }
}

impl GitTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message = input("Commit message", window, cx, |t, w, cx| t.commit(w, cx));
        let branch_name = input("New branch name", window, cx, |t, w, cx| {
            let name = value(&t.branch_name, cx).trim().to_string();
            t.switch(name, true, w, cx)
        });
        let mut tool = Self {
            dir: ctx.dir.clone(),
            root: None,
            status: Status::default(),
            selected: None,
            diff: Vec::new(),
            history: None,
            branches: None,
            message,
            branch_name,
            note: None,
            busy: false,
            clear_on_success: None,
            clear_message: None,
            task: None,
            focus: cx.focus_handle(),
        };
        tool.reload(cx);
        tool
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let dir = self.dir.clone();
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || {
                let root = repo::root(&dir)?;
                Some((root.clone(), repo::status(&root)))
            },
            |t, res, cx| {
                t.busy = false;
                match res {
                    None => t.root = None,
                    Some((root, Ok(status))) => {
                        t.root = Some(root);
                        t.status = status;
                        if t.selected
                            .as_ref()
                            .is_none_or(|s| !t.status.changes.iter().any(|c| &c.path == s))
                        {
                            t.selected = t.status.changes.first().map(|c| c.path.clone());
                        }
                        t.load_diff(cx);
                    }
                    Some((root, Err(e))) => {
                        t.root = Some(root);
                        t.note = Some((e, true));
                    }
                }
            },
        ));
    }

    fn load_diff(&mut self, _cx: &mut Context<Self>) {
        self.diff.clear();
        let (Some(root), Some(sel)) = (&self.root, &self.selected) else {
            return;
        };
        if let Some(c) = self.status.changes.iter().find(|c| &c.path == sel) {
            self.diff = repo::diff(root, c).unwrap_or_else(|e| {
                vec![DiffLine {
                    kind: LineKind::Meta,
                    old: None,
                    new: None,
                    text: e,
                }]
            });
        }
    }

    fn act(
        &mut self,
        work: impl FnOnce(&std::path::Path) -> Result<String, String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.root.clone() else {
            return;
        };
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || work(&root),
            |t, res, cx| {
                if let Some(sent) = t.clear_on_success.take()
                    && res.is_ok()
                {
                    t.clear_message = Some(sent);
                }
                t.busy = false;
                t.note = Some(match res {
                    Ok(s) => (s, false),
                    Err(e) => (e, true),
                });
                t.history = None;
                t.branches = None;
                t.reload(cx);
                cx.emit(ToolEvent::Changed);
            },
        ));
        cx.notify();
    }

    fn stage(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        self.act(move |r| repo::stage(r, &paths).map(|_| String::new()), cx);
    }

    fn unstage(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        self.act(move |r| repo::unstage(r, &paths).map(|_| String::new()), cx);
    }

    fn discard(&mut self, change: Change, cx: &mut Context<Self>) {
        self.act(
            move |r| repo::discard(r, &change).map(|_| format!("Discarded {}.", change.path)),
            cx,
        );
    }

    fn commit(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let msg = value(&self.message, cx);
        if !self.status.changes.iter().any(Change::is_staged) {
            self.note = Some(("Stage some changes first.".into(), true));
            cx.notify();
            return;
        }
        self.clear_on_success = Some(msg.clone());
        self.act(
            move |r| repo::commit(r, &msg).map(|h| format!("Committed {h}.")),
            cx,
        );
    }

    fn switch(
        &mut self,
        branch: String,
        create: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if branch.is_empty() {
            return;
        }
        self.branch_name
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.act(
            move |r| repo::checkout(r, &branch, create).map(|_| format!("Switched to {branch}.")),
            cx,
        );
    }

    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        if self.history.is_some() {
            self.history = None;
        } else if let Some(root) = &self.root {
            self.history = Some(repo::log(root, 50).unwrap_or_default());
        }
        cx.notify();
    }

    fn toggle_branches(&mut self, cx: &mut Context<Self>) {
        if self.branches.is_some() {
            self.branches = None;
        } else if let Some(root) = &self.root {
            self.branches = Some(repo::branches(root).unwrap_or_default());
        }
        cx.notify();
    }

    fn change_row(
        &self,
        ix: usize,
        c: &Change,
        staged: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let on = self.selected.as_deref() == Some(c.path.as_str());
        let path = c.path.clone();
        let (dir, name) = match path.rsplit_once('/') {
            Some((d, n)) => (d.to_string(), n.to_string()),
            None => (String::new(), path.clone()),
        };
        let p2 = path.clone();
        let change = c.clone();
        div()
            .id((if staged { "staged" } else { "unstaged" }, ix))
            .group("change")
            .h(px(28.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .rounded(px(5.0))
            .when(on, |d| d.bg(with_alpha(p.accent, 0.22)))
            .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.04))))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected = Some(p2.clone());
                this.load_diff(cx);
                cx.notify();
            }))
            .child(
                div()
                    .w(px(14.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(hex(kind_color(c)))
                    .child(
                        if staged { c.staged } else { c.unstaged }
                            .to_string()
                            .replace('?', "U"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .gap(px(6.0))
                    .text_size(px(12.5))
                    .child(div().flex_none().text_color(p.text_strong).child(name))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(p.text_muted)
                            .child(dir),
                    ),
            )
            .when(!staged, |d| {
                d.child(
                    div()
                        .id(("discard", ix))
                        .invisible()
                        .group_hover("change", |s| s.visible())
                        .text_size(px(11.0))
                        .text_color(p.text_muted)
                        .hover(|s| s.text_color(hex(0xff6961)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.discard(change.clone(), cx)
                        }))
                        .child("Discard"),
                )
            })
            .child(
                div()
                    .id((if staged { "unstage" } else { "stage" }, ix))
                    .px(px(6.0))
                    .rounded(px(4.0))
                    .text_size(px(14.0))
                    .text_color(p.text_muted)
                    .hover(|s| s.bg(white_alpha(0.1)).text_color(p.text_strong))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if staged {
                            this.unstage(vec![path.clone()], cx)
                        } else {
                            this.stage(vec![path.clone()], cx)
                        }
                    }))
                    .child(if staged { "\u{2212}" } else { "+" }),
            )
            .into_any_element()
    }

    fn section(
        &self,
        title: &'static str,
        staged: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let items: Vec<(usize, Change)> = self
            .status
            .changes
            .iter()
            .filter(|c| {
                if staged {
                    c.is_staged()
                } else {
                    c.is_unstaged()
                }
            })
            .cloned()
            .enumerate()
            .collect();
        if items.is_empty() {
            return None;
        }
        let all: Vec<String> = items.iter().map(|(_, c)| c.path.clone()).collect();
        Some(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px(px(10.0))
                        .py(px(6.0))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.text_muted)
                        .child(div().flex_1().child(format!(
                            "{} ({})",
                            title.to_uppercase(),
                            items.len()
                        )))
                        .child(
                            div()
                                .id(if staged { "unstage-all" } else { "stage-all" })
                                .cursor_pointer()
                                .hover(|s| s.text_color(p.text_strong))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if staged {
                                        this.unstage(all.clone(), cx)
                                    } else {
                                        this.stage(all.clone(), cx)
                                    }
                                }))
                                .child(if staged { "Unstage all" } else { "Stage all" }),
                        ),
                )
                .children(
                    items
                        .iter()
                        .map(|(i, c)| self.change_row(*i, c, staged, p, cx)),
                ),
        )
    }

    fn render_diff(&self, p: &Palette) -> AnyElement {
        if let Some(log) = &self.history {
            return div()
                .id("git-log")
                .size_full()
                .overflow_y_scroll()
                .children(log.iter().map(|c| {
                    div()
                        .flex()
                        .gap(px(10.0))
                        .px(px(12.0))
                        .py(px(6.0))
                        .border_b_1()
                        .border_color(white_alpha(0.05))
                        .text_size(px(12.0))
                        .child(
                            div()
                                .font_family(mono_font())
                                .text_color(p.accent)
                                .child(c.hash.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.text_strong)
                                .child(c.subject.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_color(p.text_muted)
                                .child(format!("{}, {}", c.author, c.when)),
                        )
                }))
                .into_any_element();
        }
        if self.diff.is_empty() {
            return empty("git-compare", "Pick a change to see its diff.", p).into_any_element();
        }
        div()
            .id("git-diff")
            .size_full()
            .overflow_scroll()
            .py(px(4.0))
            .font_family(mono_font())
            .text_size(px(12.0))
            .children(self.diff.iter().take(4000).map(|l| {
                let (bg, fg, sign) = match l.kind {
                    LineKind::Added => (with_alpha(hex(0x34c759), 0.14), hex(0xc8f7d2), "+"),
                    LineKind::Removed => {
                        (with_alpha(hex(0xff453a), 0.14), hex(0xffc9c5), "\u{2212}")
                    }
                    LineKind::Hunk => (with_alpha(p.accent, 0.12), p.accent, ""),
                    LineKind::Meta => (transparent_black(), p.text_dim, ""),
                    LineKind::Context => (transparent_black(), p.text, " "),
                };
                let num = |n: Option<u32>| n.map(|n| n.to_string()).unwrap_or_default();
                div()
                    .flex()
                    .bg(bg)
                    .h(px(19.0))
                    .items_center()
                    .when(l.kind != LineKind::Hunk && l.kind != LineKind::Meta, |d| {
                        d.child(
                            div()
                                .w(px(40.0))
                                .flex_none()
                                .text_right()
                                .pr(px(6.0))
                                .text_color(p.text_dim)
                                .child(num(l.old)),
                        )
                        .child(
                            div()
                                .w(px(40.0))
                                .flex_none()
                                .text_right()
                                .pr(px(8.0))
                                .text_color(p.text_dim)
                                .child(num(l.new)),
                        )
                        .child(div().w(px(14.0)).flex_none().text_color(fg).child(sign))
                    })
                    .when(matches!(l.kind, LineKind::Hunk | LineKind::Meta), |d| {
                        d.pl(px(12.0))
                    })
                    .child(
                        div()
                            .text_color(fg)
                            .whitespace_nowrap()
                            .child(l.text.replace('\t', "    ")),
                    )
            }))
            .into_any_element()
    }
}

impl Focusable for GitTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for GitTool {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(sent) = self.clear_message.take()
            && value(&self.message, cx) == sent
        {
            self.message.update(cx, |s, cx| s.set_value("", window, cx));
        }
        let p = palette(cx);
        let Some(root) = self.root.clone() else {
            return div()
                .track_focus(&self.focus)
                .size_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(header("Git", format!("{}", self.dir.display()), &p))
                .child(list_box(&p).child(empty(
                    "git-branch",
                    if self.busy {
                        "Looking for a repository\u{2026}"
                    } else {
                        "This folder isn't inside a Git repository."
                    },
                    &p,
                )))
                .into_any_element();
        };
        let s = &self.status;
        let mut sync = String::new();
        if s.ahead > 0 {
            sync.push_str(&format!("\u{2191}{} ", s.ahead));
        }
        if s.behind > 0 {
            sync.push_str(&format!("\u{2193}{}", s.behind));
        }
        let staged = self.section("Staged", true, &p, cx);
        let unstaged = self.section("Changes", false, &p, cx);
        let clean = staged.is_none() && unstaged.is_none();
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
                    .child(div().flex_1().min_w_0().child(header(
                        "Git",
                        root.display().to_string(),
                        &p,
                    )))
                    .child(
                        button(
                            "git-branch",
                            format!("{}  {}", s.branch, sync.trim()),
                            Tone::Plain,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_branches(cx))),
                    )
                    .child(
                        button("git-pull", "Pull", Tone::Plain, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.act(
                                    |r| {
                                        repo::pull(r).map(|s| {
                                            if s.is_empty() { "Pulled.".into() } else { s }
                                        })
                                    },
                                    cx,
                                )
                            },
                        )),
                    )
                    .child(
                        button("git-push", "Push", Tone::Plain, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.act(|r| repo::push(r).map(|_| "Pushed.".into()), cx)
                            },
                        )),
                    )
                    .child(
                        button(
                            "git-history",
                            if self.history.is_some() {
                                "Diff"
                            } else {
                                "History"
                            },
                            Tone::Plain,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_history(cx))),
                    ),
            )
            .when_some(self.branches.clone(), |d, list| {
                d.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(6.0))
                        .items_center()
                        .children(list.into_iter().enumerate().map(|(i, b)| {
                            let current = b == self.status.branch;
                            let name = b.clone();
                            chip(("branch", i), "", current, &p)
                                .child(b)
                                .on_click(cx.listener(move |this, _, w, cx| {
                                    this.branches = None;
                                    this.switch(name.clone(), false, w, cx)
                                }))
                        }))
                        .child(
                            div()
                                .w(px(200.0))
                                .flex()
                                .child(field(&self.branch_name, &p)),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        list_box(&p)
                            .flex_none()
                            .w(px(320.0))
                            .child(
                                div()
                                    .id("git-changes")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p(px(4.0))
                                    .when(clean, |d| {
                                        d.child(empty(
                                            "git-branch",
                                            "Nothing to commit. The tree is clean.",
                                            &p,
                                        ))
                                    })
                                    .children(staged)
                                    .children(unstaged),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .p(px(8.0))
                                    .border_t_1()
                                    .border_color(white_alpha(0.08))
                                    .child(field(&self.message, &p))
                                    .child(
                                        button("git-commit", "Commit", Tone::Primary, &p)
                                            .justify_center()
                                            .on_click(
                                                cx.listener(|this, _, w, cx| this.commit(w, cx)),
                                            ),
                                    ),
                            ),
                    )
                    .child(list_box(&p).child(self.render_diff(&p))),
            )
            .children(self.note.clone().map(|(s, e)| status_line(s, e, &p)))
            .into_any_element()
    }
}
