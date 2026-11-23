use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::{fmt_size, row_date_text};

use crate::actions::*;
use crate::icons::file_icon;
use crate::theme::{Palette, black_alpha, palette, white_alpha, with_alpha};

pub const ROW_HEIGHT: f32 = 30.0;
const DATE_COL: f32 = 180.0;
const SIZE_COL: f32 = 85.0;

#[derive(Clone)]
pub struct Row {
    pub name: SharedString,
    pub is_dir: bool,
    pub is_parent: bool,
    pub size: Option<u64>,
    pub mtime: String,
}

pub struct FilePane {
    pub path: PathBuf,
    pub rows: Vec<Row>,
    all_rows: Vec<Row>,
    pub cursor: usize,
    pub marked: Vec<bool>,
    pub focus: FocusHandle,
    pub error: Option<SharedString>,
    pub load_time: Duration,
    pub dual: bool,
    filter: String,
    scroll: UniformListScrollHandle,
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    bench: Option<Bench>,
}

struct Bench {
    last: Instant,
    frames: Vec<f64>,
}

impl FilePane {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            path: PathBuf::new(),
            rows: Vec::new(),
            all_rows: Vec::new(),
            cursor: 0,
            marked: Vec::new(),
            focus: cx.focus_handle(),
            error: None,
            load_time: Duration::ZERO,
            dual: true,
            filter: String::new(),
            scroll: UniformListScrollHandle::new(),
            back: Vec::new(),
            forward: Vec::new(),
            bench: None,
        };
        pane.load(path, None);
        pane
    }

    pub fn start_bench(&mut self, cx: &mut Context<Self>) {
        self.bench = Some(Bench {
            last: Instant::now(),
            frames: Vec::new(),
        });
        cx.notify();
    }

    pub fn load(&mut self, path: PathBuf, focus_name: Option<&str>) {
        let started = Instant::now();
        match oryn_core::list_dir(&path) {
            Ok(items) => {
                let mut rows = Vec::with_capacity(items.len() + 1);
                if path.parent().is_some() {
                    rows.push(Row {
                        name: "..".into(),
                        is_dir: true,
                        is_parent: true,
                        size: None,
                        mtime: String::new(),
                    });
                }
                rows.extend(items.into_iter().map(|it| Row {
                    name: it.name.into(),
                    is_dir: it.is_dir,
                    is_parent: false,
                    size: it.size,
                    mtime: it.mtime,
                }));
                self.all_rows = rows;
                self.path = path;
                self.error = None;
                self.filter.clear();
                self.apply_filter(focus_name);
            }
            Err(err) => {
                self.error = Some(format!("Can't open this folder: {err}").into());
            }
        }
        self.load_time = started.elapsed();
    }

    fn apply_filter(&mut self, focus_name: Option<&str>) {
        let needle = self.filter.to_lowercase();
        self.rows = if needle.is_empty() {
            self.all_rows.clone()
        } else {
            self.all_rows
                .iter()
                .filter(|r| r.is_parent || r.name.to_lowercase().contains(&needle))
                .cloned()
                .collect()
        };
        self.marked = vec![false; self.rows.len()];
        self.cursor = focus_name
            .and_then(|n| self.rows.iter().position(|r| r.name.as_ref() == n))
            .unwrap_or(0);
        self.scroll
            .scroll_to_item(self.cursor, ScrollStrategy::Center);
    }

    pub fn set_filter(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.filter == text {
            return;
        }
        self.filter = text.to_string();
        self.apply_filter(None);
        cx.notify();
    }

    pub fn navigate(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path == self.path {
            return;
        }
        self.back.push(self.path.clone());
        self.forward.clear();
        self.load(path, None);
        cx.notify();
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.back.pop() {
            self.forward.push(self.path.clone());
            self.load(prev, None);
            cx.notify();
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.forward.pop() {
            self.back.push(self.path.clone());
            self.load(next, None);
            cx.notify();
        }
    }

    pub fn title(&self) -> SharedString {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
            .into()
    }

    pub fn marked_summary(&self) -> (usize, u64) {
        self.marked
            .iter()
            .zip(&self.rows)
            .filter(|(m, _)| **m)
            .fold((0, 0), |(n, s), (_, r)| (n + 1, s + r.size.unwrap_or(0)))
    }

    pub fn totals(&self) -> (usize, u64) {
        self.rows
            .iter()
            .filter(|r| !r.is_parent)
            .fold((0, 0), |(n, s), r| (n + 1, s + r.size.unwrap_or(0)))
    }

    fn set_cursor(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        self.cursor = ix.min(self.rows.len() - 1);
        self.scroll
            .scroll_to_item(self.cursor, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn page_rows(&self, window: &Window) -> usize {
        ((window.viewport_size().height.as_f32() - 260.0) / ROW_HEIGHT).max(5.0) as usize
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(self.cursor.saturating_sub(1), cx);
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(self.cursor + 1, cx);
    }

    fn page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.page_rows(window);
        self.set_cursor(self.cursor.saturating_sub(n), cx);
    }

    fn page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.page_rows(window);
        self.set_cursor(self.cursor + n, cx);
    }

    fn home(&mut self, _: &MoveHome, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(0, cx);
    }

    fn end(&mut self, _: &MoveEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.set_cursor(usize::MAX, cx);
    }

    fn toggle_mark(&mut self, _: &ToggleMark, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get(self.cursor)
            && !row.is_parent
        {
            self.marked[self.cursor] = !self.marked[self.cursor];
        }
        self.set_cursor(self.cursor + 1, cx);
    }

    fn mark_all(&mut self, _: &MarkAll, _: &mut Window, cx: &mut Context<Self>) {
        let all = self
            .rows
            .iter()
            .zip(&self.marked)
            .all(|(r, m)| r.is_parent || *m);
        for (m, r) in self.marked.iter_mut().zip(&self.rows) {
            *m = !all && !r.is_parent;
        }
        cx.notify();
    }

    fn open_entry(&mut self, _: &OpenEntry, _: &mut Window, cx: &mut Context<Self>) {
        self.open_at(self.cursor, cx);
    }

    fn open_at(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(ix).cloned() else {
            return;
        };
        if row.is_parent {
            self.go_parent(cx);
        } else if row.is_dir {
            let target = self.path.join(row.name.as_ref());
            self.navigate(target, cx);
        }
    }

    pub fn go_parent(&mut self, cx: &mut Context<Self>) {
        let Some(parent) = self.path.parent().map(Path::to_path_buf) else {
            return;
        };
        let from = self.title().to_string();
        self.back.push(self.path.clone());
        self.forward.clear();
        self.load(parent, Some(&from));
        cx.notify();
    }

    fn render_header(&self, p: &Palette) -> impl IntoElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.0))
            .h(px(31.0))
            .px(px(16.0))
            .border_b_1()
            .border_color(p.divider)
            .bg(white_alpha(0.02))
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.text_muted)
            .child(div().flex_1().child("NAME"))
            .child(div().w(px(DATE_COL)).text_right().child("DATE MODIFIED"))
            .child(div().w(px(SIZE_COL)).text_right().child("SIZE"))
    }

    fn render_row(
        &self,
        ix: usize,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let row = &self.rows[ix];
        let marked = self.marked.get(ix).copied().unwrap_or(false);
        let is_cursor = ix == self.cursor;
        let (name_color, meta_color) = if marked {
            (p.on_accent, p.on_accent)
        } else if is_cursor {
            (p.text_strong, p.text_secondary)
        } else {
            (p.text, p.text_secondary)
        };
        let date = if row.is_parent {
            String::new()
        } else {
            row_date_text(&row.mtime)
        };
        let size = if row.is_parent {
            "\u{2014}".to_string()
        } else if row.is_dir {
            "<DIR>".to_string()
        } else {
            fmt_size(row.size)
        };
        let label = if row.is_parent {
            "Parent folder".to_string()
        } else if row.is_dir {
            format!("Folder {}, modified {}", row.name, date)
        } else {
            format!("{}, {}, modified {}", row.name, size, date)
        };

        div()
            .id(ix)
            .role(Role::Row)
            .aria_label(SharedString::from(label))
            .aria_selected(marked || is_cursor)
            .flex()
            .items_center()
            .gap(px(12.0))
            .w_full()
            .h(px(ROW_HEIGHT))
            .px(px(16.0))
            .rounded(px(6.0))
            .when(marked && is_cursor, |d| d.bg(p.accent_hover))
            .when(marked && !is_cursor, |d| {
                d.bg(p.accent).shadow(vec![BoxShadow {
                    color: with_alpha(p.accent, 0.35),
                    offset: point(px(0.0), px(2.0)),
                    blur_radius: px(8.0),
                    spread_radius: px(0.0),
                    inset: false,
                }])
            })
            .when(!marked && is_cursor && active, |d| {
                d.bg(p.accent_soft)
                    .border_1()
                    .border_color(with_alpha(p.accent, 0.45))
            })
            .when(!marked && is_cursor && !active, |d| d.bg(white_alpha(0.06)))
            .when(!marked && !is_cursor, |d| {
                d.hover(|s| s.bg(white_alpha(0.04)))
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                window.focus(&this.focus, cx);
                this.set_cursor(ix, cx);
                if ev.click_count() >= 2 {
                    this.open_at(ix, cx);
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(13.0))
                    .text_color(name_color)
                    .child(file_icon(&row.name, row.is_dir, marked, p))
                    .child(div().truncate().child(row.name.clone())),
            )
            .child(
                div()
                    .w(px(DATE_COL))
                    .flex_none()
                    .text_right()
                    .text_size(px(12.0))
                    .text_color(meta_color)
                    .child(date),
            )
            .child(
                div()
                    .w(px(SIZE_COL))
                    .flex_none()
                    .text_right()
                    .text_size(px(12.0))
                    .text_color(meta_color)
                    .child(size),
            )
    }

    fn bench_tick(&mut self, window: &mut Window) {
        let Some(bench) = self.bench.as_mut() else {
            return;
        };
        let now = Instant::now();
        bench
            .frames
            .push(now.duration_since(bench.last).as_secs_f64() * 1000.0);
        bench.last = now;
        if self.cursor + 1 >= self.rows.len() || bench.frames.len() >= 600 {
            let mut f = bench.frames.split_off(1);
            f.sort_by(|a, b| a.total_cmp(b));
            let avg = f.iter().sum::<f64>() / f.len().max(1) as f64;
            let p95 = f.get(f.len() * 95 / 100).copied().unwrap_or(0.0);
            println!(
                "bench rows={} load_ms={:.1} frames={} avg_frame_ms={:.2} p95_frame_ms={:.2}",
                self.rows.len(),
                self.load_time.as_secs_f64() * 1000.0,
                f.len(),
                avg,
                p95
            );
            self.bench = None;
            return;
        }
        self.cursor = (self.cursor + 173).min(self.rows.len() - 1);
        self.scroll
            .scroll_to_item(self.cursor, ScrollStrategy::Center);
        window.request_animation_frame();
    }
}

impl Render for FilePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.bench_tick(window);
        let p = palette(cx);
        let active = self.focus.contains_focused(window, cx);
        let highlight = active && self.dual;
        let count = self.rows.len();
        let message = |text: SharedString| {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.0))
                .text_color(p.text_muted)
                .child(text)
                .into_any_element()
        };
        let body: AnyElement = if let Some(err) = &self.error {
            message(err.clone())
        } else if count <= 1 && !self.filter.is_empty() {
            message(format!("Nothing here matches \u{201c}{}\u{201d}", self.filter).into())
        } else if count == 0 {
            message("This folder is empty".into())
        } else {
            uniform_list(
                "rows",
                count,
                cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                    let p = palette(cx);
                    let active = this.focus.contains_focused(window, cx);
                    range
                        .map(|ix| this.render_row(ix, active, &p, cx))
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .flex_1()
            .into_any_element()
        };

        div()
            .id("pane")
            .key_context("FilePane")
            .track_focus(&self.focus)
            .role(Role::List)
            .aria_label(SharedString::from(format!(
                "Files in {}",
                self.path.display()
            )))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::toggle_mark))
            .on_action(cx.listener(Self::mark_all))
            .on_action(cx.listener(Self::open_entry))
            .on_action(cx.listener(|this, _: &GoParent, _, cx| this.go_parent(cx)))
            .on_action(cx.listener(|this, _: &GoBack, _, cx| this.go_back(cx)))
            .on_action(cx.listener(|this, _: &GoForward, _, cx| this.go_forward(cx)))
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(p.card)
            .rounded(px(10.0))
            .border_1()
            .overflow_hidden()
            .when(highlight, |d| {
                d.border_color(with_alpha(p.accent, 0.65)).shadow(vec![
                    BoxShadow {
                        color: with_alpha(p.accent, 0.3),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(1.0),
                        inset: false,
                    },
                    BoxShadow {
                        color: black_alpha(0.4),
                        offset: point(px(0.0), px(8.0)),
                        blur_radius: px(24.0),
                        spread_radius: px(0.0),
                        inset: false,
                    },
                ])
            })
            .when(!highlight, |d| {
                d.border_color(p.card_border).shadow(vec![BoxShadow {
                    color: black_alpha(0.25),
                    offset: point(px(0.0), px(4.0)),
                    blur_radius: px(20.0),
                    spread_radius: px(0.0),
                    inset: false,
                }])
            })
            .when(self.dual && !active, |d| d.opacity(0.92))
            .child(self.render_header(&p))
            .child(body)
    }
}
