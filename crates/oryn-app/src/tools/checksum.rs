use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use gpui_kit::component::input::InputState;
use oryn_core::format::fmt_size;
use oryn_core::tools::checksum::{Sums, hash_file, matches};
use parking_lot::Mutex;

use super::*;
use crate::icons::file_icon;
use crate::theme::{mono_font, palette};

type Snapshot = (PathBuf, u64, Option<Result<Sums, String>>);

struct Item {
    path: PathBuf,
    size: u64,
    sums: Option<Result<Sums, String>>,
}

pub struct ChecksumTool {
    items: Arc<Mutex<Vec<Item>>>,
    done: Arc<AtomicU64>,
    total: u64,
    cancel: Arc<AtomicBool>,
    expected: Entity<InputState>,
    busy: bool,
    task: Option<Task<()>>,
    copied: Option<String>,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for ChecksumTool {}

impl ChecksumTool {
    pub fn new(ctx: &ToolContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let expected = input("Paste a checksum to verify", window, cx, |_, _, cx| {
            cx.notify()
        });
        cx.observe(&expected, |_, _, cx| cx.notify()).detach();
        let items: Vec<Item> = ctx
            .targets
            .iter()
            .filter(|p| p.is_file())
            .map(|p| Item {
                path: p.clone(),
                size: p.metadata().map(|m| m.len()).unwrap_or(0),
                sums: None,
            })
            .collect();
        let total = items.iter().map(|i| i.size).sum();
        let mut tool = Self {
            items: Arc::new(Mutex::new(items)),
            done: Arc::default(),
            total,
            cancel: Arc::default(),
            expected,
            busy: false,
            task: None,
            copied: None,
            focus: cx.focus_handle(),
        };
        tool.start(cx);
        tool
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        if self.items.lock().is_empty() {
            return;
        }
        let items = self.items.clone();
        let done = self.done.clone();
        let cancel = self.cancel.clone();
        self.busy = true;
        self.task = Some(run_bg(
            cx,
            move || {
                let n = items.lock().len();
                for i in 0..n {
                    let path = items.lock()[i].path.clone();
                    let res = match hash_file(&path, &done, &cancel) {
                        Ok(Some(s)) => Ok(s),
                        Ok(None) => return,
                        Err(e) => Err(e.to_string()),
                    };
                    items.lock()[i].sums = Some(res);
                }
            },
            |t, _, _| t.busy = false,
        ));
    }

    fn copy(&mut self, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        self.copied = Some(text);
        cx.notify();
    }

    fn hash_row(
        &self,
        id: (&'static str, usize),
        algo: &'static str,
        sum: &str,
        hit: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let text = sum.to_string();
        let copied = self.copied.as_deref() == Some(sum);
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .when(hit, |d| d.bg(with_alpha(hex(0x34c759), 0.16)))
            .hover(|s| s.bg(white_alpha(0.05)))
            .on_click(cx.listener(move |this, _, _, cx| this.copy(text.clone(), cx)))
            .child(
                div()
                    .w(px(60.0))
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(if hit { hex(0x34c759) } else { p.text_muted })
                    .child(algo),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(mono_font())
                    .text_size(px(12.0))
                    .text_color(p.text_strong)
                    .child(sum.to_string()),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(p.text_muted)
                    .child(if copied { "Copied" } else { "Copy" }),
            )
    }
}

impl Focusable for ChecksumTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Drop for ChecksumTool {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Render for ChecksumTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let expected = value(&self.expected, cx);
        let done = self.done.load(Ordering::Relaxed);
        let fraction = if self.total == 0 {
            1.0
        } else {
            done as f32 / self.total as f32
        };
        let rows: Vec<Snapshot> = self
            .items
            .lock()
            .iter()
            .map(|i| (i.path.clone(), i.size, i.sums.clone()))
            .collect();
        let verdict = rows.iter().find_map(|(_, _, s)| match s {
            Some(Ok(s)) => matches(s, &expected).filter(|m| !m.is_empty()),
            _ => None,
        });
        let any_done = rows.iter().any(|(_, _, s)| matches!(s, Some(Ok(_))));
        let cards = rows
            .into_iter()
            .enumerate()
            .map(|(ix, (path, size, sums))| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let body = match &sums {
                    None => div()
                        .px(px(8.0))
                        .text_size(px(12.0))
                        .text_color(p.text_muted)
                        .child("Hashing\u{2026}"),
                    Some(Err(e)) => div()
                        .px(px(8.0))
                        .text_size(px(12.0))
                        .text_color(hex(0xff6961))
                        .child(e.clone()),
                    Some(Ok(s)) => {
                        let m = matches(s, &expected);
                        div()
                            .flex()
                            .flex_col()
                            .child(self.hash_row(
                                ("sha256", ix),
                                "SHA-256",
                                &s.sha256,
                                m == Some("SHA-256"),
                                &p,
                                cx,
                            ))
                            .child(self.hash_row(
                                ("sha512", ix),
                                "SHA-512",
                                &s.sha512,
                                m == Some("SHA-512"),
                                &p,
                                cx,
                            ))
                    }
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .p(px(10.0))
                    .border_b_1()
                    .border_color(white_alpha(0.06))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(file_icon(&name, false, false, &p))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.0))
                                    .text_color(p.text_strong)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(p.text_muted)
                                    .child(fmt_size(Some(size))),
                            ),
                    )
                    .child(body)
            });
        let empty_state = self.items.lock().is_empty();
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(header(
                "Checksums",
                "SHA-256 and SHA-512 for the selected files. Click a value to copy it.",
                &p,
            ))
            .child(div().flex().gap(px(8.0)).child(field(&self.expected, &p)))
            .when(self.busy, |d| {
                d.child(
                    div()
                        .h(px(4.0))
                        .w_full()
                        .rounded_full()
                        .bg(white_alpha(0.08))
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .bg(p.accent)
                                .w(relative(fraction.clamp(0.02, 1.0))),
                        ),
                )
            })
            .child(if empty_state {
                list_box(&p).child(empty(
                    "hash",
                    "Select one or more files in the pane, then open Checksums.",
                    &p,
                ))
            } else {
                list_box(&p).child(
                    div()
                        .id("sums")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(cards),
                )
            })
            .when(!expected.trim().is_empty() && any_done, |d| {
                d.child(match verdict {
                    Some(algo) => status_line(
                        format!("Match. The {algo} checksum is the same."),
                        false,
                        &p,
                    )
                    .text_color(hex(0x34c759)),
                    None if self.busy => status_line("Still hashing\u{2026}", false, &p),
                    None => status_line(
                        "No match. The file differs from the checksum you pasted.",
                        true,
                        &p,
                    ),
                })
            })
    }
}
