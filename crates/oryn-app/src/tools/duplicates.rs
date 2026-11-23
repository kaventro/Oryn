use std::collections::BTreeSet;
use std::path::PathBuf;

use oryn_core::batch::{self, BatchKind, Progress};
use oryn_core::format::{fmt_size, row_date_text};
use oryn_core::services::fs_duplicates::{DuplicateGroup, DuplicateScanOptions, scan_duplicates};
use oryn_core::services::fs_transfer::OverwritePolicy;

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

pub struct DuplicatesTool {
    root: PathBuf,
    min_size: u64,
    groups: Vec<DuplicateGroup>,
    scanned: u64,
    marked: BTreeSet<String>,
    busy: bool,
    note: Option<(String, bool)>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for DuplicatesTool {}

enum Line {
    Group(usize),
    File(usize, usize),
}

impl DuplicatesTool {
    pub fn new(ctx: &ToolContext, _: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            root: ctx.dir.clone(),
            min_size: 1024,
            groups: Vec::new(),
            scanned: 0,
            marked: BTreeSet::new(),
            busy: false,
            note: None,
            task: None,
            focus: cx.focus_handle(),
        }
    }

    fn scan(&mut self, cx: &mut Context<Self>) {
        let opts = DuplicateScanOptions {
            path: self.root.to_string_lossy().into_owned(),
            min_size_bytes: Some(self.min_size),
            max_results: Some(2000),
        };
        self.busy = true;
        self.marked.clear();
        self.note = None;
        self.task = Some(run_bg(
            cx,
            move || scan_duplicates(opts).map_err(|e| e.to_string()),
            |t, res, _| {
                t.busy = false;
                match res {
                    Ok(r) => {
                        t.scanned = r.total_scanned;
                        t.note = Some((
                            if r.duplicate_groups.is_empty() {
                                format!("No duplicates among {} files.", r.total_scanned)
                            } else {
                                format!(
                                    "{} sets of duplicates, {} that could be freed.",
                                    r.duplicate_groups.len(),
                                    fmt_size(Some(r.total_wasted_bytes))
                                )
                            },
                            false,
                        ));
                        t.groups = r.duplicate_groups;
                    }
                    Err(e) => t.note = Some((e, true)),
                }
            },
        ));
        cx.notify();
    }

    fn keep_one(&mut self, newest: bool, cx: &mut Context<Self>) {
        self.marked.clear();
        for g in &self.groups {
            let keep = if newest {
                g.files.iter().max_by(|a, b| a.mtime.cmp(&b.mtime))
            } else {
                g.files.iter().min_by(|a, b| a.mtime.cmp(&b.mtime))
            }
            .map(|f| f.path.clone());
            for f in &g.files {
                if Some(&f.path) != keep.as_ref() {
                    self.marked.insert(f.path.clone());
                }
            }
        }
        cx.notify();
    }

    fn trash_marked(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<PathBuf> = self.marked.iter().map(PathBuf::from).collect();
        if paths.is_empty() {
            return;
        }
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || {
                let report = batch::run(
                    BatchKind::Trash,
                    &paths,
                    None,
                    OverwritePolicy::Overwrite,
                    &Progress::default(),
                );
                (paths.len(), report.failed.len())
            },
            |t, (n, failed), cx| {
                t.busy = false;
                cx.emit(ToolEvent::Changed);
                t.scan(cx);
                t.note = Some(if failed == 0 {
                    (
                        format!(
                            "Moved {n} file{} to the Trash.",
                            if n == 1 { "" } else { "s" }
                        ),
                        false,
                    )
                } else {
                    (
                        format!("{failed} of {n} couldn't be moved to the Trash."),
                        true,
                    )
                });
            },
        ));
        cx.notify();
    }

    fn lines(&self) -> Vec<Line> {
        let mut v = Vec::new();
        for (gi, g) in self.groups.iter().enumerate() {
            v.push(Line::Group(gi));
            for fi in 0..g.files.len() {
                v.push(Line::File(gi, fi));
            }
        }
        v
    }

    fn render_line(
        &self,
        ix: usize,
        line: &Line,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match *line {
            Line::Group(gi) => {
                let g = &self.groups[gi];
                div()
                    .id(("dup-g", ix))
                    .w_full()
                    .h(px(30.0))
                    .px(px(12.0))
                    .flex()
                    .items_end()
                    .pb(px(4.0))
                    .gap(px(8.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_muted)
                    .child(format!(
                        "{} COPIES OF {}",
                        g.files.len(),
                        fmt_size(Some(g.size)).to_uppercase()
                    ))
                    .child(div().flex_1())
                    .child(format!("{} wasted", fmt_size(Some(g.total_wasted))))
                    .into_any_element()
            }
            Line::File(gi, fi) => {
                let f = &self.groups[gi].files[fi];
                let on = self.marked.contains(&f.path);
                let path = f.path.clone();
                let reveal = PathBuf::from(&f.path);
                let parent = reveal
                    .parent()
                    .map(|d| rel_to(d, &self.root))
                    .unwrap_or_default();
                div()
                    .id(("dup-f", ix))
                    .w_full()
                    .h(px(30.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .hover(|s| s.bg(white_alpha(0.04)))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                        if ev.click_count() > 1 {
                            cx.emit(ToolEvent::Reveal(reveal.clone()));
                        } else if !this.marked.remove(&path) {
                            this.marked.insert(path.clone());
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(14.0))
                            .flex_none()
                            .rounded(px(3.0))
                            .border_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.0))
                            .when(on, |d| {
                                d.bg(hex(0xff453a))
                                    .border_color(hex(0xff453a))
                                    .text_color(hex(0xffffff))
                                    .child("\u{2713}")
                            })
                            .when(!on, |d| d.border_color(white_alpha(0.3))),
                    )
                    .child(file_icon(&f.name, false, false, p))
                    .child(
                        div()
                            .flex_none()
                            .text_color(if on { p.text_muted } else { p.text_strong })
                            .child(f.name.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(p.text_muted)
                            .child(parent),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.0))
                            .text_color(p.text_muted)
                            .child(row_date_text(&f.mtime)),
                    )
                    .into_any_element()
            }
        }
    }
}

impl Focusable for DuplicatesTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for DuplicatesTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let sizes = [
            (1u64, "Any size"),
            (1024, "Over 1 KB"),
            (1024 * 1024, "Over 1 MB"),
            (100 * 1024 * 1024, "Over 100 MB"),
        ];
        let lines = self.lines();
        let n = lines.len();
        let marked = self.marked.len();
        let freed: u64 = self
            .groups
            .iter()
            .flat_map(|g| g.files.iter().map(move |f| (g.size, f)))
            .filter(|(_, f)| self.marked.contains(&f.path))
            .map(|(s, _)| s)
            .sum();
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
                        "Duplicates",
                        format!("Files with identical contents in {}", self.root.display()),
                        &p,
                    )))
                    .child(
                        button(
                            "dup-scan",
                            if self.busy {
                                "Scanning\u{2026}"
                            } else {
                                "Scan"
                            },
                            Tone::Primary,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.scan(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .children(sizes.into_iter().enumerate().map(|(i, (size, label))| {
                        chip(("dup-size", i), label, self.min_size == size, &p).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.min_size = size;
                                cx.notify();
                            }),
                        )
                    }))
                    .child(div().flex_1())
                    .when(!self.groups.is_empty(), |d| {
                        d.child(
                            button("dup-newest", "Keep newest", Tone::Plain, &p)
                                .on_click(cx.listener(|this, _, _, cx| this.keep_one(true, cx))),
                        )
                        .child(
                            button("dup-oldest", "Keep oldest", Tone::Plain, &p)
                                .on_click(cx.listener(|this, _, _, cx| this.keep_one(false, cx))),
                        )
                        .child(
                            button(
                                "dup-trash",
                                if marked == 0 {
                                    "Move to Trash".to_string()
                                } else {
                                    format!("Move {marked} to Trash ({})", fmt_size(Some(freed)))
                                },
                                Tone::Danger,
                                &p,
                            )
                            .when(marked == 0, |d| d.opacity(0.5))
                            .on_click(cx.listener(|this, _, _, cx| this.trash_marked(cx))),
                        )
                    }),
            )
            .child(if n == 0 {
                list_box(&p).child(empty(
                    "files",
                    if self.busy {
                        "Hashing files\u{2026}"
                    } else {
                        "Press Scan to look for files with the same contents."
                    },
                    &p,
                ))
            } else {
                list_box(&p).child(
                    uniform_list(
                        "dup-lines",
                        n,
                        cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                            let p = palette(cx);
                            let lines = this.lines();
                            range
                                .filter_map(|i| {
                                    lines.get(i).map(|l| this.render_line(i, l, &p, cx))
                                })
                                .collect::<Vec<_>>()
                        }),
                    )
                    .flex_1(),
                )
            })
            .children(self.note.clone().map(|(s, e)| status_line(s, e, &p)))
    }
}
