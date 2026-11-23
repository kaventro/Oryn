use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui_kit::component::input::InputState;
use oryn_core::format::fmt_size;
use oryn_core::tools::find::{FindQuery, Hit, find};
use parking_lot::Mutex;

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

pub struct SearchTool {
    root: PathBuf,
    name: Entity<InputState>,
    content: Entity<InputState>,
    case: bool,
    regex: bool,
    hidden: bool,
    hits: Arc<Mutex<Vec<Hit>>>,
    cancel: Arc<AtomicBool>,
    running: bool,
    status: Option<(String, bool)>,
    selected: usize,
    task: Option<Task<()>>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for SearchTool {}

impl SearchTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = input("File name, like *.png or report", window, cx, |t, _, cx| {
            t.start(cx)
        });
        let content = input("Text inside files (optional)", window, cx, |t, _, cx| {
            t.start(cx)
        });
        let first = name.clone();
        cx.defer_in(window, move |_, window, cx| {
            first.update(cx, |s, cx| s.focus(window, cx));
        });
        Self {
            root: ctx.dir.clone(),
            name,
            content,
            case: false,
            regex: false,
            hidden: false,
            hits: Arc::default(),
            cancel: Arc::default(),
            running: false,
            status: None,
            selected: 0,
            task: None,
            focus: cx.focus_handle(),
        }
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        self.cancel.store(true, Ordering::Relaxed);
        let q = FindQuery {
            root: self.root.clone(),
            name: value(&self.name, cx).trim().to_string(),
            content: value(&self.content, cx),
            case_sensitive: self.case,
            regex: self.regex,
            hidden: self.hidden,
            limit: 5000,
        };
        if q.name.is_empty() && q.content.is_empty() {
            self.status = Some(("Type a name or some text to search for.".into(), true));
            cx.notify();
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();
        self.hits = Arc::default();
        self.selected = 0;
        self.running = true;
        self.status = None;
        let hits = self.hits.clone();
        self.task = Some(run_bg(
            cx,
            move || find(&q, &cancel, |h| hits.lock().push(h)),
            |t, res, _| {
                t.running = false;
                let n = t.hits.lock().len();
                t.status = Some(match res {
                    Ok(_) if n == 0 => ("Nothing found.".into(), false),
                    Ok(_) if n >= 5000 => ("Showing the first 5,000 matches.".into(), false),
                    Ok(_) => (
                        format!("{n} match{}.", if n == 1 { "" } else { "es" }),
                        false,
                    ),
                    Err(e) => (e, true),
                });
            },
        ));
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.cancel.store(true, Ordering::Relaxed);
        cx.notify();
    }

    fn open(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(hit) = self.hits.lock().get(ix).cloned() else {
            return;
        };
        cx.emit(if hit.is_dir {
            ToolEvent::Open(hit.path)
        } else {
            ToolEvent::Reveal(hit.path)
        });
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.hits.lock().len();
        if n > 0 {
            self.selected = (self.selected as isize + delta).clamp(0, n as isize - 1) as usize;
            cx.notify();
        }
    }

    fn render_hit(&self, ix: usize, hit: &Hit, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let on = ix == self.selected;
        let name = hit
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent = hit
            .path
            .parent()
            .map(|d| rel_to(d, &self.root))
            .unwrap_or_default();
        div()
            .id(("hit", ix))
            .w_full()
            .h(px(44.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .cursor_pointer()
            .when(on, |d| d.bg(with_alpha(p.accent, 0.22)))
            .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.04))))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                this.selected = ix;
                if ev.click_count() > 1 {
                    this.open(ix, cx);
                }
                cx.notify();
            }))
            .child(file_icon(&name, hit.is_dir, false, p))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(p.text_strong)
                            .truncate()
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(p.text_muted)
                            .truncate()
                            .child(match &hit.line {
                                Some((n, text)) => format!("{n}: {text}"),
                                None if parent.is_empty() => ".".to_string(),
                                None => parent,
                            }),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(p.text_muted)
                    .child(if hit.is_dir {
                        "Folder".to_string()
                    } else {
                        fmt_size(Some(hit.size))
                    }),
            )
            .into_any_element()
    }
}

impl Focusable for SearchTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SearchTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let count = self.hits.lock().len();
        let status = if self.running {
            Some((format!("Searching\u{2026} {count} found"), false))
        } else {
            self.status.clone()
        };
        div()
            .id("search-tool")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "up" => this.step(-1, cx),
                    "down" => this.step(1, cx),
                    "enter"
                        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control =>
                    {
                        let ix = this.selected;
                        this.open(ix, cx)
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(header("Search", format!("In {}", self.root.display()), &p))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(field(&self.name, &p))
                    .child(field(&self.content, &p))
                    .child(if self.running {
                        button("search-stop", "Stop", Tone::Plain, &p)
                            .on_click(cx.listener(|this, _, _, cx| this.stop(cx)))
                    } else {
                        button("search-go", "Search", Tone::Primary, &p)
                            .on_click(cx.listener(|this, _, _, cx| this.start(cx)))
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .items_center()
                    .child(
                        chip("s-case", "Match case", self.case, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.case = !this.case;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        chip("s-regex", "Regex", self.regex, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.regex = !this.regex;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        chip("s-hidden", "Hidden files", self.hidden, &p).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.hidden = !this.hidden;
                                cx.notify();
                            },
                        )),
                    )
                    .child(div().flex_1())
                    .children(status.map(|(s, e)| status_line(s, e, &p))),
            )
            .child(if count == 0 {
                list_box(&p).child(empty(
                    "search",
                    if self.running {
                        "Searching\u{2026}"
                    } else {
                        "Results show up here. Double-click one to show it in the pane."
                    },
                    &p,
                ))
            } else {
                list_box(&p).child(
                    uniform_list(
                        "hits",
                        count,
                        cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                            let p = palette(cx);
                            let hits: Vec<(usize, Hit)> = {
                                let all = this.hits.lock();
                                range
                                    .filter_map(|i| all.get(i).cloned().map(|h| (i, h)))
                                    .collect()
                            };
                            hits.iter()
                                .map(|(i, h)| this.render_hit(*i, h, &p, cx))
                                .collect::<Vec<_>>()
                        }),
                    )
                    .flex_1(),
                )
            })
    }
}
