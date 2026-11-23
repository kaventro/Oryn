use std::path::PathBuf;

use gpui_kit::component::input::InputState;
use oryn_core::format::fmt_size;
use oryn_core::tools::dircmp::{self, Direction, Row, State, Step};

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

pub struct CompareTool {
    left: Entity<InputState>,
    right: Entity<InputState>,
    by_content: bool,
    show_same: bool,
    direction: Direction,
    mirror: bool,
    rows: Vec<Row>,
    roots: Option<(PathBuf, PathBuf)>,
    pending: Option<Vec<Step>>,
    busy: bool,
    note: Option<(String, bool)>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for CompareTool {}

fn state_color(s: State) -> u32 {
    match s {
        State::Same => 0x8e8e93,
        State::LeftNewer | State::RightNewer => 0xff9f0a,
        State::Differs => 0xff453a,
        State::LeftOnly => 0x0a84ff,
        State::RightOnly => 0xbf5af2,
    }
}

impl CompareTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let left = input("Left folder", window, cx, |t, _, cx| t.compare(cx));
        let right = input("Right folder", window, cx, |t, _, cx| t.compare(cx));
        let (l, r) = (
            ctx.dir.to_string_lossy().into_owned(),
            ctx.other.to_string_lossy().into_owned(),
        );
        left.update(cx, |s, cx| s.set_value(l, window, cx));
        right.update(cx, |s, cx| s.set_value(r, window, cx));
        let mut tool = Self {
            left,
            right,
            by_content: false,
            show_same: false,
            direction: Direction::LeftToRight,
            mirror: false,
            rows: Vec::new(),
            roots: None,
            pending: None,
            busy: false,
            note: None,
            task: None,
            focus: cx.focus_handle(),
        };
        if ctx.dir != ctx.other {
            tool.compare(cx);
        }
        tool
    }

    fn compare(&mut self, cx: &mut Context<Self>) {
        let l = PathBuf::from(value(&self.left, cx).trim());
        let r = PathBuf::from(value(&self.right, cx).trim());
        if !l.is_dir() || !r.is_dir() {
            self.note = Some(("Both sides need to be folders.".into(), true));
            cx.notify();
            return;
        }
        if l == r {
            self.note = Some(("Pick two different folders.".into(), true));
            cx.notify();
            return;
        }
        let by_content = self.by_content;
        self.busy = true;
        self.pending = None;
        self.note = None;
        let roots = (l.clone(), r.clone());
        self.task = Some(run_bg(
            cx,
            move || dircmp::compare(&l, &r, by_content),
            move |t, res, _| {
                t.busy = false;
                match res {
                    Ok(rows) => {
                        let diff = rows.iter().filter(|r| r.state != State::Same).count();
                        t.note = Some((
                            if diff == 0 {
                                format!("The folders match. {} items compared.", rows.len())
                            } else {
                                format!("{diff} of {} items differ.", rows.len())
                            },
                            false,
                        ));
                        t.rows = rows;
                        t.roots = Some(roots);
                    }
                    Err(e) => t.note = Some((e, true)),
                }
            },
        ));
        cx.notify();
    }

    fn preview_sync(&mut self, cx: &mut Context<Self>) {
        let Some((l, r)) = &self.roots else {
            return;
        };
        let steps = dircmp::plan(&self.rows, l, r, self.direction, self.mirror);
        if steps.is_empty() {
            self.note = Some(("Nothing to sync in that direction.".into(), false));
            self.pending = None;
        } else {
            self.pending = Some(steps);
        }
        cx.notify();
    }

    fn apply_sync(&mut self, cx: &mut Context<Self>) {
        let Some(steps) = self.pending.take() else {
            return;
        };
        self.busy = true;
        let n = steps.len();
        self.task = Some(run_bg(
            cx,
            move || dircmp::apply(&steps),
            move |t, failed, cx| {
                t.busy = false;
                cx.emit(ToolEvent::Changed);
                t.compare(cx);
                t.note = Some(if failed.is_empty() {
                    (
                        format!("Synced {n} item{}.", if n == 1 { "" } else { "s" }),
                        false,
                    )
                } else {
                    (
                        format!(
                            "{} of {n} failed. First: {}: {}",
                            failed.len(),
                            failed[0].0,
                            failed[0].1
                        ),
                        true,
                    )
                });
            },
        ));
        cx.notify();
    }

    fn render_row(&self, ix: usize, row: &Row, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let name = row.rel.rsplit('/').next().unwrap_or(&row.rel).to_string();
        let depth = row.rel.matches('/').count();
        let side = |size: Option<u64>, dir: bool| match size {
            None => String::new(),
            Some(_) if dir => "Folder".into(),
            s => fmt_size(s),
        };
        let reveal = self.roots.as_ref().map(|(l, r)| {
            if row.left_size.is_some() {
                l.join(&row.rel)
            } else {
                r.join(&row.rel)
            }
        });
        div()
            .id(("cmp", ix))
            .w_full()
            .h(px(28.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(12.5))
            .hover(|s| s.bg(white_alpha(0.04)))
            .on_click(cx.listener(move |_, ev: &ClickEvent, _, cx| {
                if ev.click_count() > 1
                    && let Some(p) = &reveal
                {
                    cx.emit(ToolEvent::Reveal(p.clone()));
                }
            }))
            .child(div().w(px(depth as f32 * 14.0)).flex_none())
            .child(file_icon(&name, row.is_dir, false, p))
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
                    .w(px(80.0))
                    .text_right()
                    .text_color(p.text_muted)
                    .child(side(row.left_size, row.is_dir)),
            )
            .child(
                div().w(px(130.0)).flex().justify_center().child(
                    div()
                        .px(px(8.0))
                        .rounded(px(9.0))
                        .text_size(px(10.5))
                        .bg(with_alpha(hex(state_color(row.state)), 0.18))
                        .text_color(hex(state_color(row.state)))
                        .child(row.state.label()),
                ),
            )
            .child(
                div()
                    .w(px(80.0))
                    .text_color(p.text_muted)
                    .child(side(row.right_size, row.is_dir)),
            )
            .into_any_element()
    }
}

impl Focusable for CompareTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CompareTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let visible: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| self.show_same || r.state != State::Same)
            .map(|(i, _)| i)
            .collect();
        let dirs = [
            (Direction::LeftToRight, "Left \u{2192} Right"),
            (Direction::RightToLeft, "Left \u{2190} Right"),
            (Direction::Both, "Both ways"),
        ];
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(header(
                "Compare and Sync",
                "Compare two folders, then copy what's missing or newer.",
                &p,
            ))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(field(&self.left, &p))
                    .child(field(&self.right, &p))
                    .child(
                        button(
                            "cmp-go",
                            if self.busy {
                                "Working\u{2026}"
                            } else {
                                "Compare"
                            },
                            Tone::Primary,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.compare(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        chip("cmp-content", "Compare contents", self.by_content, &p).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.by_content = !this.by_content;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        chip("cmp-same", "Show matching", self.show_same, &p).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.show_same = !this.show_same;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(div().flex_1())
                    .children(dirs.into_iter().enumerate().map(|(i, (d, label))| {
                        chip(("cmp-dir", i), label, self.direction == d, &p).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.direction = d;
                                this.pending = None;
                                cx.notify();
                            },
                        ))
                    }))
                    .child(
                        chip("cmp-mirror", "Delete extras", self.mirror, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.mirror = !this.mirror;
                                this.pending = None;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        button("cmp-sync", "Sync\u{2026}", Tone::Plain, &p)
                            .on_click(cx.listener(|this, _, _, cx| this.preview_sync(cx))),
                    ),
            )
            .when_some(self.pending.as_ref(), |d, steps| {
                let copies = steps
                    .iter()
                    .filter(|s| matches!(s, Step::Copy { .. }))
                    .count();
                let removes = steps.len() - copies;
                let mut text = format!(
                    "Sync will copy {copies} item{}",
                    if copies == 1 { "" } else { "s" }
                );
                if removes > 0 {
                    text.push_str(&format!(" and delete {removes}"));
                }
                text.push_str(". Files that exist on the target side are replaced.");
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(with_alpha(p.accent, 0.12))
                        .border_1()
                        .border_color(with_alpha(p.accent, 0.35))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.5))
                                .text_color(p.text_strong)
                                .child(text),
                        )
                        .child(button("cmp-cancel", "Cancel", Tone::Plain, &p).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.pending = None;
                                cx.notify();
                            }),
                        ))
                        .child(
                            button(
                                "cmp-apply",
                                "Sync now",
                                if removes > 0 {
                                    Tone::Danger
                                } else {
                                    Tone::Primary
                                },
                                &p,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.apply_sync(cx))),
                        ),
                )
            })
            .child(if visible.is_empty() {
                list_box(&p).child(empty(
                    "git-compare",
                    if self.busy {
                        "Comparing\u{2026}"
                    } else if self.rows.is_empty() {
                        "Pick two folders and press Compare."
                    } else {
                        "Everything matches."
                    },
                    &p,
                ))
            } else {
                list_box(&p).child(
                    uniform_list(
                        "cmp-rows",
                        visible.len(),
                        cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                            let p = palette(cx);
                            range
                                .filter_map(|i| visible.get(i).copied())
                                .map(|ix| {
                                    let row = this.rows[ix].clone();
                                    this.render_row(ix, &row, &p, cx)
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
