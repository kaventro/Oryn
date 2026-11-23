use std::path::PathBuf;

use oryn_core::format::fmt_size;
use oryn_core::services::fs_size::{DiskSpaceAnalysis, analyze_directory};

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

const BAR_COLORS: [u32; 6] = [0x0a84ff, 0x5ac8fa, 0x34c759, 0xff9f0a, 0xbf5af2, 0xff375f];

pub struct DiskTool {
    root: PathBuf,
    trail: Vec<PathBuf>,
    data: Option<DiskSpaceAnalysis>,
    busy: bool,
    error: Option<String>,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for DiskTool {}

impl DiskTool {
    pub fn new(ctx: &ToolContext, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut tool = Self {
            root: ctx.dir.clone(),
            trail: vec![ctx.dir.clone()],
            data: None,
            busy: false,
            error: None,
            task: None,
            focus: cx.focus_handle(),
        };
        tool.load(cx);
        tool
    }

    fn current(&self) -> PathBuf {
        self.trail
            .last()
            .cloned()
            .unwrap_or_else(|| self.root.clone())
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let dir = self.current();
        self.busy = true;
        self.error = None;
        self.task = Some(run_bg(
            cx,
            move || analyze_directory(&dir).map_err(|e| e.to_string()),
            |t, res, _| {
                t.busy = false;
                match res {
                    Ok(d) => t.data = Some(d),
                    Err(e) => t.error = Some(e),
                }
            },
        ));
        cx.notify();
    }

    fn enter(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.trail.push(path);
        self.load(cx);
    }

    fn back_to(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.trail.truncate(ix + 1);
        self.load(cx);
    }
}

impl Focusable for DiskTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for DiskTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let crumbs = self.trail.iter().enumerate().map(|(i, path)| {
            let label = if i == 0 {
                path.display().to_string()
            } else {
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            let last = i + 1 == self.trail.len();
            div()
                .id(("disk-crumb", i))
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(12.0))
                .text_color(if last { p.text_strong } else { p.text_muted })
                .when(!last, |d| {
                    d.cursor_pointer()
                        .hover(|s| s.text_color(p.text_strong))
                        .on_click(cx.listener(move |this, _, _, cx| this.back_to(i, cx)))
                })
                .when(i > 0, |d| {
                    d.child(div().text_color(p.text_dim).child("\u{203a}"))
                })
                .child(label)
        });
        let data = self
            .data
            .as_ref()
            .filter(|d| std::path::Path::new(&d.path) == self.current());
        let total = data.map(|d| d.total_size).unwrap_or(0).max(1);
        let top = data
            .and_then(|d| d.items.first())
            .map(|i| i.size)
            .unwrap_or(1)
            .max(1);
        let rows: Vec<AnyElement> = data
            .map(|d| {
                d.items
                    .iter()
                    .take(300)
                    .enumerate()
                    .map(|(ix, item)| {
                        let share = item.size as f32 / total as f32;
                        let width = item.size as f32 / top as f32;
                        let color = hex(BAR_COLORS[ix % BAR_COLORS.len()]);
                        let path = PathBuf::from(&item.path);
                        let is_dir = item.is_dir;
                        div()
                            .id(("disk", ix))
                            .w_full()
                            .h(px(38.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(white_alpha(0.04)))
                            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                                if is_dir && ev.click_count() == 1 {
                                    this.enter(path.clone(), cx);
                                } else if !is_dir || ev.click_count() > 1 {
                                    cx.emit(ToolEvent::Reveal(path.clone()));
                                }
                            }))
                            .child(file_icon(&item.name, item.is_dir, false, &p))
                            .child(
                                div()
                                    .w(px(220.0))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.5))
                                            .text_color(p.text_strong)
                                            .child(item.name.clone()),
                                    )
                                    .child(
                                        div().text_size(px(10.5)).text_color(p.text_muted).child(
                                            if item.is_dir {
                                                format!("{} files", item.files)
                                            } else {
                                                "File".into()
                                            },
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(10.0))
                                    .rounded_full()
                                    .bg(white_alpha(0.05))
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .h_full()
                                            .rounded_full()
                                            .bg(color)
                                            .w(relative(width.max(0.005))),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(70.0))
                                    .text_right()
                                    .text_size(px(12.0))
                                    .text_color(p.text_strong)
                                    .child(fmt_size(Some(item.size))),
                            )
                            .child(
                                div()
                                    .w(px(48.0))
                                    .text_right()
                                    .text_size(px(11.0))
                                    .text_color(p.text_muted)
                                    .child(format!("{:.1}%", share * 100.0)),
                            )
                            .into_any_element()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let summary = data
            .map(|d| {
                format!(
                    "{} in {} files and {} folders",
                    fmt_size(Some(d.total_size)),
                    d.total_files,
                    d.total_dirs
                )
            })
            .unwrap_or_default();
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
                    .child(div().flex_1().child(header("Disk Space", summary, &p)))
                    .child(
                        button(
                            "disk-reload",
                            if self.busy {
                                "Measuring\u{2026}"
                            } else {
                                "Refresh"
                            },
                            Tone::Plain,
                            &p,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                    ),
            )
            .child(div().flex().flex_wrap().gap(px(6.0)).children(crumbs))
            .child(if rows.is_empty() {
                list_box(&p).child(empty(
                    "chart-pie",
                    match (&self.error, self.busy) {
                        (Some(e), _) => e.clone(),
                        (None, true) => "Measuring folder sizes\u{2026}".into(),
                        (None, false) => "This folder is empty.".into(),
                    },
                    &p,
                ))
            } else {
                list_box(&p).child(
                    div()
                        .id("disk-rows")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(rows),
                )
            })
            .child(status_line(
                "Click a folder to look inside it. Double-click to show it in the pane.",
                false,
                &p,
            ))
    }
}
