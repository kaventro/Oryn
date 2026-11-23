use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::{fmt_size, rfc3339_millis, row_date_text};
use oryn_core::ui::arrange::{ArrangeEntry, SortField, SortOrder, arrange, short_extension};
use oryn_core::ui::selection::{Entry, Selection};

use crate::actions::*;
use crate::icons::file_icon;
use crate::theme::{Palette, black_alpha, palette, white_alpha, with_alpha};

pub const ROW_HEIGHT: f32 = 30.0;
const TILE_MIN: f32 = 96.0;
const TILE_HEIGHT: f32 = 88.0;
const TILE_GAP: f32 = 6.0;
const GRID_PAD: f32 = 10.0;
const COLUMN_WIDTH: f32 = 240.0;
const COLUMN_ROW: f32 = 26.0;
const INSPECTOR_WIDTH: f32 = 280.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    List,
    Grid,
    Columns,
}
const DATE_COL: f32 = 180.0;
const SIZE_COL: f32 = 85.0;

#[derive(Clone)]
pub struct Row {
    pub name: SharedString,
    pub is_dir: bool,
    pub is_parent: bool,
    pub size: Option<u64>,
    pub mtime: String,
    pub mtime_ms: i64,
}

impl ArrangeEntry for Row {
    fn base(&self) -> &str {
        if self.is_parent { ".." } else { &self.name }
    }
    fn is_dir(&self) -> bool {
        self.is_dir
    }
    fn size(&self) -> u64 {
        self.size.unwrap_or(0)
    }
    fn mtime_ms(&self) -> i64 {
        self.mtime_ms
    }
}

impl Entry for Row {
    fn base(&self) -> &str {
        ArrangeEntry::base(self)
    }
    fn is_dir(&self) -> bool {
        self.is_dir
    }
    fn size(&self) -> u64 {
        self.size.unwrap_or(0)
    }
}

struct Column {
    path: PathBuf,
    rows: Vec<Row>,
    cursor: Option<usize>,
    scroll: UniformListScrollHandle,
}

impl Column {
    fn new(path: PathBuf, rows: Vec<Row>) -> Self {
        Self {
            path,
            rows,
            cursor: None,
            scroll: UniformListScrollHandle::new(),
        }
    }

    fn selected(&self) -> Option<&Row> {
        self.cursor.and_then(|i| self.rows.get(i))
    }
}

fn read_rows(path: &Path) -> anyhow::Result<Vec<Row>> {
    Ok(oryn_core::list_dir(path)?
        .into_iter()
        .map(|it| Row {
            name: it.name.into(),
            is_dir: it.is_dir,
            is_parent: false,
            size: it.size,
            mtime_ms: rfc3339_millis(&it.mtime),
            mtime: it.mtime,
        })
        .collect())
}

pub struct FilePane {
    pub path: PathBuf,
    pub rows: Vec<Row>,
    all_rows: Vec<Row>,
    pub cursor: usize,
    pub selection: Selection,
    pub order: SortOrder,
    pub focus: FocusHandle,
    pub error: Option<SharedString>,
    pub load_time: Duration,
    pub dual: bool,
    pub view: ViewMode,
    pub notice: Option<SharedString>,
    grid_width: f32,
    columns: Vec<Column>,
    col_ix: usize,
    col_scroll: ScrollHandle,
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
            selection: Selection::default(),
            order: SortOrder::default(),
            focus: cx.focus_handle(),
            error: None,
            load_time: Duration::ZERO,
            dual: true,
            view: ViewMode::List,
            notice: None,
            grid_width: 0.0,
            columns: Vec::new(),
            col_ix: 0,
            col_scroll: ScrollHandle::new(),
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
        match read_rows(&path) {
            Ok(items) => {
                let mut rows = Vec::with_capacity(items.len() + 1);
                if path.parent().is_some() {
                    rows.push(Row {
                        name: "..".into(),
                        is_dir: true,
                        is_parent: true,
                        size: None,
                        mtime: String::new(),
                        mtime_ms: 0,
                    });
                }
                rows.extend(items);
                self.all_rows = rows;
                self.path = path;
                self.error = None;
                self.filter.clear();
                self.selection = Selection::default();
                self.notice = None;
                self.apply_filter(focus_name);
            }
            Err(err) => {
                self.error = Some(format!("Can't open this folder: {err}").into());
            }
        }
        self.load_time = started.elapsed();
    }

    fn apply_filter(&mut self, focus_name: Option<&str>) {
        let order = arrange(&self.all_rows, &self.filter, self.order);
        self.rows = order
            .into_iter()
            .map(|i| self.all_rows[i].clone())
            .collect();
        self.cursor = focus_name
            .and_then(|n| self.rows.iter().position(|r| r.name.as_ref() == n))
            .unwrap_or(0);
        self.scroll
            .scroll_to_item(self.scroll_target(), ScrollStrategy::Center);
        self.reset_columns();
    }

    pub fn set_view(&mut self, view: ViewMode) {
        if self.view != view {
            self.view = view;
            self.reset_columns();
        }
    }

    fn arranged(&self, rows: Vec<Row>) -> Vec<Row> {
        let order = arrange(&rows, "", self.order);
        let mut slots: Vec<Option<Row>> = rows.into_iter().map(Some).collect();
        order.into_iter().filter_map(|i| slots[i].take()).collect()
    }

    fn reset_columns(&mut self) {
        if self.view != ViewMode::Columns {
            self.columns.clear();
            self.col_ix = 0;
            return;
        }
        let same_root = self.columns.first().is_some_and(|c| c.path == self.path);
        let chain: Vec<SharedString> = if same_root {
            self.columns
                .iter()
                .map_while(|c| c.selected().map(|r| r.name.clone()))
                .collect()
        } else {
            self.rows
                .get(self.cursor)
                .filter(|r| !r.is_parent)
                .map(|r| vec![r.name.clone()])
                .unwrap_or_default()
        };
        let keep_ix = if same_root { self.col_ix } else { 0 };
        let root: Vec<Row> = self.rows.iter().filter(|r| !r.is_parent).cloned().collect();
        self.columns = vec![Column::new(self.path.clone(), root)];
        for (depth, name) in chain.iter().enumerate() {
            let Some(col) = self.columns.get_mut(depth) else {
                break;
            };
            let Some(ix) = col.rows.iter().position(|r| &r.name == name) else {
                break;
            };
            col.cursor = Some(ix);
            col.scroll.scroll_to_item(ix, ScrollStrategy::Center);
            self.expand_from(depth);
        }
        self.col_ix = keep_ix.min(self.columns.len() - 1);
    }

    fn expand_from(&mut self, ci: usize) {
        self.columns.truncate(ci + 1);
        let Some(col) = self.columns.get(ci) else {
            return;
        };
        let Some(row) = col.selected().filter(|r| r.is_dir) else {
            return;
        };
        let child = col.path.join(row.name.as_ref());
        if let Ok(rows) = read_rows(&child) {
            let rows = self.arranged(rows);
            self.columns.push(Column::new(child, rows));
        }
    }

    fn col_select(&mut self, ci: usize, ri: usize, cx: &mut Context<Self>) {
        let Some(col) = self.columns.get_mut(ci) else {
            return;
        };
        if col.rows.is_empty() {
            return;
        }
        let ri = ri.min(col.rows.len() - 1);
        col.cursor = Some(ri);
        col.scroll.scroll_to_item(ri, ScrollStrategy::Nearest);
        let name = col.rows[ri].name.clone();
        self.col_ix = ci;
        self.expand_from(ci);
        if ci == 0
            && let Some(ix) = self
                .rows
                .iter()
                .position(|r| !r.is_parent && r.name == name)
        {
            self.cursor = ix;
        }
        let last = self.columns.len() - 1;
        let tail = if self.columns[last].selected().is_some_and(|r| !r.is_dir) {
            last + 1
        } else {
            last
        };
        self.col_scroll.scroll_to_item(tail);
        cx.notify();
    }

    fn col_current(&self) -> Option<usize> {
        self.columns.get(self.col_ix).and_then(|c| c.cursor)
    }

    fn col_len(&self) -> usize {
        self.columns.get(self.col_ix).map_or(0, |c| c.rows.len())
    }

    fn col_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let next = match self.col_current() {
            None => 0,
            Some(cur) => cur.saturating_add_signed(delta),
        };
        self.col_select(self.col_ix, next, cx);
    }

    fn col_left(&mut self, cx: &mut Context<Self>) {
        if self.col_ix == 0 {
            self.go_parent(cx);
            return;
        }
        self.col_ix -= 1;
        self.columns.truncate(self.col_ix + 2);
        if let Some(child) = self.columns.get_mut(self.col_ix + 1) {
            child.cursor = None;
        }
        cx.notify();
    }

    fn col_right(&mut self, cx: &mut Context<Self>) {
        let has_child = self
            .columns
            .get(self.col_ix)
            .and_then(Column::selected)
            .is_some_and(|r| r.is_dir);
        if has_child
            && self
                .columns
                .get(self.col_ix + 1)
                .is_some_and(|c| !c.rows.is_empty())
        {
            self.col_select(self.col_ix + 1, 0, cx);
        }
    }

    fn col_open(&mut self, cx: &mut Context<Self>) {
        let Some(col) = self.columns.get(self.col_ix) else {
            return;
        };
        if let Some(row) = col.selected().filter(|r| r.is_dir) {
            let target = col.path.join(row.name.as_ref());
            self.navigate(target, cx);
        }
    }

    pub fn sort_by(&mut self, field: SortField, cx: &mut Context<Self>) {
        self.order = self.order.toggled(field);
        let current = self.rows.get(self.cursor).map(|r| r.name.to_string());
        self.apply_filter(current.as_deref());
        cx.notify();
    }

    fn report(&mut self, text: String, cx: &mut Context<Self>) {
        self.notice = Some(text.into());
        cx.notify();
    }

    pub fn apply_mask(&mut self, mask: &str, selecting: bool, cx: &mut Context<Self>) {
        let text = self.selection.apply_mask(&self.all_rows, mask, selecting);
        self.report(text, cx);
    }

    fn invert_selection(&mut self, _: &InvertSelection, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.selection.invert(&self.all_rows);
        self.report(text, cx);
    }

    fn clear_selection(&mut self, _: &ClearSelection, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.selection.clear();
        self.report(text, cx);
    }

    fn select_by_extension(
        &mut self,
        _: &SelectByExtension,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor = self.rows.get(self.cursor);
        let text = self
            .selection
            .select_by_extension(&self.all_rows, cursor)
            .unwrap_or_else(|| "Put the cursor on a file first.".to_string());
        self.report(text, cx);
    }

    pub fn is_marked(&self, ix: usize) -> bool {
        self.rows
            .get(ix)
            .is_some_and(|r| !r.is_parent && self.selection.contains(&r.name))
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
        let sum = self.selection.summary(&self.all_rows);
        (sum.sel_count, sum.sel_bytes)
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
            .scroll_to_item(self.scroll_target(), ScrollStrategy::Nearest);
        cx.notify();
    }

    fn page_rows(&self, window: &Window) -> usize {
        ((window.viewport_size().height.as_f32() - 260.0) / ROW_HEIGHT).max(5.0) as usize
    }

    fn grid_cols(&self) -> usize {
        let inner = (self.grid_width - GRID_PAD * 2.0 + TILE_GAP).max(0.0);
        ((inner / (TILE_MIN + TILE_GAP)).floor() as usize).max(1)
    }

    fn vertical_step(&self) -> usize {
        if self.view == ViewMode::Grid {
            self.grid_cols()
        } else {
            1
        }
    }

    fn scroll_target(&self) -> usize {
        if self.view == ViewMode::Grid {
            self.cursor / self.grid_cols()
        } else {
            self.cursor
        }
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            return self.col_step(-1, cx);
        }
        let step = self.vertical_step();
        if self.cursor >= step {
            self.set_cursor(self.cursor - step, cx);
        }
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            return self.col_step(1, cx);
        }
        let step = self.vertical_step();
        if self.cursor + step < self.rows.len() {
            self.set_cursor(self.cursor + step, cx);
        }
    }

    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        match self.view {
            ViewMode::Grid => self.set_cursor(self.cursor.saturating_sub(1), cx),
            ViewMode::Columns => self.col_left(cx),
            ViewMode::List => {}
        }
    }

    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        match self.view {
            ViewMode::Grid => self.set_cursor(self.cursor + 1, cx),
            ViewMode::Columns => self.col_right(cx),
            ViewMode::List => {}
        }
    }

    fn page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.page_rows(window);
        if self.view == ViewMode::Columns {
            return self.col_step(-(n as isize), cx);
        }
        self.set_cursor(self.cursor.saturating_sub(n), cx);
    }

    fn page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        let n = self.page_rows(window);
        if self.view == ViewMode::Columns {
            return self.col_step(n as isize, cx);
        }
        self.set_cursor(self.cursor + n, cx);
    }

    fn home(&mut self, _: &MoveHome, _: &mut Window, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            return self.col_select(self.col_ix, 0, cx);
        }
        self.set_cursor(0, cx);
    }

    fn end(&mut self, _: &MoveEnd, _: &mut Window, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            return self.col_select(self.col_ix, self.col_len(), cx);
        }
        self.set_cursor(usize::MAX, cx);
    }

    fn toggle_mark(&mut self, _: &ToggleMark, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get(self.cursor)
            && !row.is_parent
        {
            let name = row.name.clone();
            self.selection.toggle(&name);
            self.notice = None;
        }
        self.set_cursor(self.cursor + 1, cx);
    }

    fn mark_all(&mut self, _: &MarkAll, _: &mut Window, cx: &mut Context<Self>) {
        let sum = self.selection.summary(&self.all_rows);
        if sum.sel_count == sum.total_files + sum.total_dirs {
            self.selection.clear();
        } else {
            self.selection.select_all(&self.all_rows);
        }
        cx.notify();
    }

    fn open_entry(&mut self, _: &OpenEntry, _: &mut Window, cx: &mut Context<Self>) {
        if self.view == ViewMode::Columns {
            return self.col_open(cx);
        }
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

    fn header_cell(
        &self,
        id: &'static str,
        label: &'static str,
        field: SortField,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let active = self.order.field == field;
        let arrow = if self.order.ascending {
            "\u{25b4}"
        } else {
            "\u{25be}"
        };
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(format!("Sort by {}", label.to_lowercase()))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .when(active, |d| d.text_color(p.text_secondary))
            .hover(|s| s.text_color(p.text_strong))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.sort_by(field, cx);
                window.focus(&this.focus, cx);
            }))
            .child(label)
            .when(active, |d| d.child(div().text_size(px(9.0)).child(arrow)))
    }

    fn render_header(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(div().flex_1().child(self.header_cell(
                "sort-name",
                "NAME",
                SortField::Name,
                p,
                cx,
            )))
            .child(
                div()
                    .w(px(DATE_COL))
                    .flex()
                    .justify_end()
                    .child(self.header_cell("sort-date", "DATE MODIFIED", SortField::Date, p, cx)),
            )
            .child(
                div()
                    .w(px(SIZE_COL))
                    .flex()
                    .justify_end()
                    .child(self.header_cell("sort-size", "SIZE", SortField::Size, p, cx)),
            )
    }

    fn render_tile(
        &self,
        ix: usize,
        width: f32,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let row = &self.rows[ix];
        let marked = self.is_marked(ix);
        let is_cursor = ix == self.cursor;
        div()
            .id(ix)
            .role(Role::ListItem)
            .aria_label(SharedString::from(if row.is_parent {
                "Parent folder".to_string()
            } else {
                row.name.to_string()
            }))
            .aria_selected(marked || is_cursor)
            .flex()
            .flex_col()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .w(px(width))
            .h(px(TILE_HEIGHT))
            .px(px(6.0))
            .pt(px(10.0))
            .pb(px(8.0))
            .rounded(px(10.0))
            .when(marked, |d| d.bg(p.accent))
            .when(!marked && is_cursor && active, |d| {
                d.bg(p.accent_soft)
                    .border_1()
                    .border_color(with_alpha(p.accent, 0.45))
            })
            .when(!marked && is_cursor && !active, |d| d.bg(white_alpha(0.08)))
            .when(!marked && !is_cursor, |d| {
                d.hover(|s| s.bg(white_alpha(0.06)))
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                window.focus(&this.focus, cx);
                this.set_cursor(ix, cx);
                if ev.click_count() >= 2 {
                    this.open_at(ix, cx);
                }
            }))
            .child(file_icon(&row.name, row.is_dir, marked, p).size(px(36.0)))
            .child(
                div()
                    .w_full()
                    .max_w(px(90.0))
                    .h(px(30.0))
                    .overflow_hidden()
                    .text_center()
                    .text_size(px(11.0))
                    .line_height(px(14.3))
                    .text_color(if marked { p.on_accent } else { p.text })
                    .child(row.name.clone()),
            )
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> AnyElement {
        let cols = self.grid_cols();
        let count = self.rows.len();
        let lines = count.div_ceil(cols);
        let width = self.grid_width;
        let tile_w =
            ((width - GRID_PAD * 2.0 - TILE_GAP * (cols as f32 - 1.0)) / cols as f32).max(TILE_MIN);
        let this = cx.entity().downgrade();
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let w = bounds.size.width.as_f32();
                        if let Some(this) = this.upgrade() {
                            this.update(cx, |pane, cx| {
                                if (pane.grid_width - w).abs() > 0.5 {
                                    pane.grid_width = w;
                                    cx.notify();
                                }
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                uniform_list(
                    "tiles",
                    lines,
                    cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                        let p = palette(cx);
                        let active = this.focus.contains_focused(window, cx);
                        range
                            .map(|line| {
                                let start = line * cols;
                                let end = (start + cols).min(this.rows.len());
                                div()
                                    .flex()
                                    .gap(px(TILE_GAP))
                                    .px(px(GRID_PAD))
                                    .pb(px(TILE_GAP))
                                    .when(line == 0, |d| d.pt(px(GRID_PAD)))
                                    .children(
                                        (start..end)
                                            .map(|ix| this.render_tile(ix, tile_w, active, &p, cx)),
                                    )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll)
                .size_full(),
            )
            .into_any_element()
    }

    fn render_column_row(
        &self,
        ci: usize,
        ri: usize,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let col = &self.columns[ci];
        let row = &col.rows[ri];
        let selected = col.cursor == Some(ri);
        let strong = selected && active && ci == self.col_ix;
        div()
            .id(ri)
            .role(Role::ListItem)
            .aria_label(row.name.clone())
            .aria_selected(selected)
            .flex()
            .items_center()
            .gap(px(6.0))
            .w_full()
            .h(px(COLUMN_ROW))
            .mb(px(1.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .when(strong, |d| d.bg(p.accent))
            .when(selected && !strong, |d| d.bg(white_alpha(0.08)))
            .when(!selected, |d| d.hover(|s| s.bg(white_alpha(0.04))))
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                window.focus(&this.focus, cx);
                this.col_select(ci, ri, cx);
                if ev.click_count() >= 2 {
                    this.col_open(cx);
                }
            }))
            .child(file_icon(&row.name, row.is_dir, strong, p))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(if strong || selected {
                        p.text_strong
                    } else {
                        p.text
                    })
                    .child(row.name.clone()),
            )
            .when(row.is_dir, |d| {
                d.child(
                    div()
                        .flex_none()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if strong { p.on_accent } else { p.text_muted })
                        .child("\u{203a}"),
                )
            })
    }

    fn render_column(
        &self,
        ci: usize,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let col = &self.columns[ci];
        let body: AnyElement = if col.rows.is_empty() {
            div()
                .flex()
                .justify_center()
                .p(px(8.0))
                .text_size(px(12.0))
                .italic()
                .text_color(p.text_muted)
                .child("Empty Folder")
                .into_any_element()
        } else {
            uniform_list(
                ("column", ci),
                col.rows.len(),
                cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                    let p = palette(cx);
                    let active = this.focus.contains_focused(window, cx);
                    let len = this.columns.get(ci).map_or(0, |c| c.rows.len());
                    range
                        .filter(|ri| *ri < len)
                        .map(|ri| this.render_column_row(ci, ri, active, &p, cx))
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(&col.scroll)
            .size_full()
            .into_any_element()
        };
        div()
            .id(("column-wrap", ci))
            .role(Role::List)
            .aria_label(SharedString::from(
                col.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| col.path.display().to_string()),
            ))
            .flex_none()
            .w(px(COLUMN_WIDTH))
            .h_full()
            .p(px(4.0))
            .border_r_1()
            .border_color(p.divider)
            .when(active && ci == self.col_ix, |d| d.bg(white_alpha(0.015)))
            .child(body)
            .into_any_element()
    }

    fn render_inspector(&self, row: &Row, p: &Palette) -> AnyElement {
        let meta = |label: &'static str, value: String| {
            div()
                .flex()
                .justify_between()
                .gap(px(8.0))
                .child(div().flex_none().text_color(p.text_muted).child(label))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(p.text_secondary)
                        .child(value),
                )
        };
        let kind = match short_extension(&row.name, false).as_str() {
            "" | "\u{2014}" => "File".to_string(),
            ext => ext.to_uppercase(),
        };
        div()
            .id("inspector")
            .flex_none()
            .w(px(INSPECTOR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .py(px(24.0))
            .px(px(16.0))
            .bg(black_alpha(0.08))
            .border_r_1()
            .border_color(p.divider)
            .child(
                div()
                    .size(px(56.0))
                    .mb(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(file_icon(&row.name, false, false, p).size(px(48.0))),
            )
            .child(
                div()
                    .mb(px(16.0))
                    .text_center()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .line_height(px(17.0))
                    .text_color(p.text_strong)
                    .child(row.name.clone()),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(white_alpha(0.04))
                    .border_1()
                    .border_color(white_alpha(0.06))
                    .text_size(px(11.5))
                    .child(meta("Kind", kind))
                    .child(meta("Size", fmt_size(row.size)))
                    .child(meta("Modified", row_date_text(&row.mtime))),
            )
            .into_any_element()
    }

    fn render_columns(&self, active: bool, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let inspector = self
            .columns
            .last()
            .and_then(Column::selected)
            .filter(|r| !r.is_dir)
            .map(|r| self.render_inspector(r, p));
        div()
            .id("columns")
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .overflow_x_scroll()
            .track_scroll(&self.col_scroll)
            .children((0..self.columns.len()).map(|ci| self.render_column(ci, active, p, cx)))
            .children(inspector)
            .into_any_element()
    }

    fn render_row(
        &self,
        ix: usize,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let row = &self.rows[ix];
        let marked = self.is_marked(ix);
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
            .scroll_to_item(self.scroll_target(), ScrollStrategy::Center);
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
        } else if self.view == ViewMode::Grid {
            self.render_grid(cx)
        } else if self.view == ViewMode::Columns {
            self.render_columns(active, &p, cx)
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
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::toggle_mark))
            .on_action(cx.listener(Self::mark_all))
            .on_action(cx.listener(Self::invert_selection))
            .on_action(cx.listener(Self::clear_selection))
            .on_action(cx.listener(Self::select_by_extension))
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
            .when(self.view == ViewMode::List, |d| {
                d.child(self.render_header(&p, cx))
            })
            .child(body)
    }
}
