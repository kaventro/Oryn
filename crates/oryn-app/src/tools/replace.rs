use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use gpui_kit::component::input::InputState;
use oryn_core::tools::textreplace::{self, FileMatch, ReplaceQuery};

use super::*;
use crate::icons::file_icon;
use crate::theme::{mono_font, palette};

pub struct ReplaceTool {
    root: PathBuf,
    find: Entity<InputState>,
    replace: Entity<InputState>,
    glob: Entity<InputState>,
    regex: bool,
    case: bool,
    hits: Vec<FileMatch>,
    skipped: BTreeSet<PathBuf>,
    query: Option<ReplaceQuery>,
    busy: bool,
    note: Option<(String, bool)>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for ReplaceTool {}

impl ReplaceTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let find = input("Find", window, cx, |t, _, cx| t.preview(cx));
        let replace = input("Replace with", window, cx, |t, _, cx| t.preview(cx));
        let glob = input("Files, like *.md (optional)", window, cx, |t, _, cx| {
            t.preview(cx)
        });
        let first = find.clone();
        cx.defer_in(window, move |_, window, cx| {
            first.update(cx, |s, cx| s.focus(window, cx));
        });
        Self {
            root: ctx.dir.clone(),
            find,
            replace,
            glob,
            regex: false,
            case: false,
            hits: Vec::new(),
            skipped: BTreeSet::new(),
            query: None,
            busy: false,
            note: None,
            task: None,
            focus: cx.focus_handle(),
        }
    }

    fn query(&self, cx: &App) -> ReplaceQuery {
        ReplaceQuery {
            root: self.root.clone(),
            find: value(&self.find, cx),
            replace: value(&self.replace, cx),
            regex: self.regex,
            case_sensitive: self.case,
            glob: value(&self.glob, cx),
        }
    }

    fn preview(&mut self, cx: &mut Context<Self>) {
        let q = self.query(cx);
        self.busy = true;
        self.note = None;
        self.skipped.clear();
        let kept = q.clone();
        self.task = Some(run_bg(
            cx,
            move || textreplace::preview(&q, &AtomicBool::new(false)),
            move |t, res, _| {
                t.busy = false;
                match res {
                    Ok(hits) => {
                        let total: usize = hits.iter().map(|h| h.count).sum();
                        t.note = Some((
                            if hits.is_empty() {
                                "No matches.".into()
                            } else {
                                format!("{total} matches in {} files.", hits.len())
                            },
                            false,
                        ));
                        t.hits = hits;
                        t.query = Some(kept);
                    }
                    Err(e) => {
                        t.hits.clear();
                        t.query = None;
                        t.note = Some((e, true));
                    }
                }
            },
        ));
        cx.notify();
    }

    fn apply(&mut self, cx: &mut Context<Self>) {
        let Some(q) = self.query.clone() else {
            return;
        };
        let targets: Vec<PathBuf> = self
            .hits
            .iter()
            .map(|h| h.path.clone())
            .filter(|p| !self.skipped.contains(p))
            .collect();
        if targets.is_empty() {
            return;
        }
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || textreplace::apply(&q, &targets),
            |t, res, cx| {
                t.busy = false;
                t.hits.clear();
                t.query = None;
                cx.emit(ToolEvent::Changed);
                t.note = Some(match res {
                    Ok((n, failed)) if failed.is_empty() => (
                        format!("Changed {n} file{}.", if n == 1 { "" } else { "s" }),
                        false,
                    ),
                    Ok((n, failed)) => (
                        format!(
                            "Changed {n} files. {} failed: {}",
                            failed.len(),
                            failed[0].1
                        ),
                        true,
                    ),
                    Err(e) => (e, true),
                });
            },
        ));
        cx.notify();
    }
}

impl Focusable for ReplaceTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ReplaceTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let chosen = self.hits.len() - self.skipped.len();
        let cards = self.hits.iter().enumerate().map(|(i, h)| {
            let on = !self.skipped.contains(&h.path);
            let name = h
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let rel = h
                .path
                .parent()
                .map(|d| rel_to(d, &self.root))
                .unwrap_or_default();
            let path = h.path.clone();
            let reveal = h.path.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(10.0))
                .border_b_1()
                .border_color(white_alpha(0.06))
                .when(!on, |d| d.opacity(0.45))
                .child(
                    div()
                        .id(("rep-file", i))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                            if ev.click_count() > 1 {
                                cx.emit(ToolEvent::Reveal(reveal.clone()));
                            } else if !this.skipped.remove(&path) {
                                this.skipped.insert(path.clone());
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .size(px(14.0))
                                .rounded(px(3.0))
                                .border_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .when(on, |d| {
                                    d.bg(p.accent)
                                        .border_color(p.accent)
                                        .text_color(p.on_accent)
                                        .child("\u{2713}")
                                })
                                .when(!on, |d| d.border_color(white_alpha(0.3))),
                        )
                        .child(file_icon(&name, false, false, &p))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.text_strong)
                                .child(name),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.5))
                                .text_color(p.text_muted)
                                .child(rel),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(p.text_muted)
                                .child(format!("{}", h.count)),
                        ),
                )
                .children(h.sample.iter().map(|(line, before, after)| {
                    div()
                        .flex()
                        .flex_col()
                        .pl(px(46.0))
                        .font_family(mono_font())
                        .text_size(px(11.5))
                        .child(
                            div()
                                .truncate()
                                .text_color(hex(0xffc9c5))
                                .child(format!("{line:>4} \u{2212} {before}")),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_color(hex(0xc8f7d2))
                                .child(format!("{line:>4} + {after}")),
                        )
                }))
        });
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(header(
                "Find and Replace",
                format!("Text files in {}", self.root.display()),
                &p,
            ))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(field(&self.find, &p))
                    .child(field(&self.replace, &p))
                    .child(div().w(px(180.0)).flex().child(field(&self.glob, &p)))
                    .child(
                        button(
                            "rep-preview",
                            if self.busy {
                                "Working\u{2026}"
                            } else {
                                "Preview"
                            },
                            Tone::Plain,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.preview(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        chip("rep-case", "Match case", self.case, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.case = !this.case;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        chip("rep-regex", "Regex", self.regex, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.regex = !this.regex;
                                cx.notify();
                            },
                        )),
                    )
                    .child(div().flex_1())
                    .children(self.note.clone().map(|(s, e)| status_line(s, e, &p)))
                    .when(chosen > 0, |d| {
                        d.child(
                            button(
                                "rep-apply",
                                format!(
                                    "Replace in {chosen} file{}",
                                    if chosen == 1 { "" } else { "s" }
                                ),
                                Tone::Primary,
                                &p,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.apply(cx))),
                        )
                    }),
            )
            .child(if self.hits.is_empty() {
                list_box(&p).child(empty(
                    "replace",
                    "Preview shows every change before anything is written.",
                    &p,
                ))
            } else {
                list_box(&p).child(
                    div()
                        .id("rep-list")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(cards),
                )
            })
    }
}
