use std::path::PathBuf;

use oryn_core::tools::tags::{COLORS, Tags};

use super::*;
use crate::icons::file_icon;
use crate::theme::palette;

pub fn color_of(name: &str) -> u32 {
    match name {
        "Red" => 0xff453a,
        "Orange" => 0xff9f0a,
        "Yellow" => 0xffd60a,
        "Green" => 0x34c759,
        "Blue" => 0x0a84ff,
        "Purple" => 0xbf5af2,
        _ => 0x8e8e93,
    }
}

pub struct TagsTool {
    tags: Tags,
    targets: Vec<PathBuf>,
    filter: String,
    pub focus: FocusHandle,
}

impl EventEmitter<ToolEvent> for TagsTool {}

impl TagsTool {
    pub fn new(ctx: &ToolContext, _: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            tags: Tags::load(),
            targets: ctx.targets.clone(),
            filter: ctx.tag.clone().unwrap_or_else(|| "Red".into()),
            focus: cx.focus_handle(),
        }
    }

    fn toggle(&mut self, color: &str, cx: &mut Context<Self>) {
        if self.targets.is_empty() {
            return;
        }
        self.tags.toggle(&self.targets, color);
        self.tags.save();
        cx.emit(ToolEvent::Changed);
        cx.notify();
    }
}

impl Focusable for TagsTool {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TagsTool {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let n = self.targets.len();
        let subject = match n {
            0 => "Nothing selected. Select files in the pane to tag them.".to_string(),
            1 => self.targets[0]
                .file_name()
                .map(|s| format!("Tag \u{201c}{}\u{201d}", s.to_string_lossy()))
                .unwrap_or_default(),
            n => format!("Tag {n} selected items"),
        };
        let swatches = COLORS.iter().enumerate().map(|(i, &c)| {
            let all = n > 0 && self.targets.iter().all(|t| self.tags.has(t, c));
            let col = hex(color_of(c));
            div()
                .id(("tag-set", i))
                .role(Role::CheckBox)
                .aria_label(c)
                .aria_toggled(if all { Toggled::True } else { Toggled::False })
                .flex()
                .items_center()
                .gap(px(7.0))
                .px(px(10.0))
                .h(px(30.0))
                .rounded(px(15.0))
                .border_1()
                .cursor_pointer()
                .text_size(px(12.5))
                .when(all, |d| {
                    d.bg(with_alpha(col, 0.2))
                        .border_color(col)
                        .text_color(p.text_strong)
                })
                .when(!all, |d| {
                    d.border_color(white_alpha(0.12))
                        .text_color(p.text)
                        .hover(|s| s.bg(white_alpha(0.06)))
                })
                .when(n == 0, |d| d.opacity(0.4))
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(c, cx)))
                .child(div().size(px(10.0)).rounded_full().bg(col))
                .child(c)
                .when(all, |d| {
                    d.child(div().text_size(px(11.0)).child("\u{2713}"))
                })
        });
        let filters = COLORS.iter().enumerate().map(|(i, &c)| {
            let on = self.filter == c;
            let col = hex(color_of(c));
            div()
                .id(("tag-filter", i))
                .size(px(22.0))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .border_2()
                .border_color(if on {
                    hex(0xffffff)
                } else {
                    transparent_black()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.filter = c.into();
                    cx.notify();
                }))
                .child(div().size(px(14.0)).rounded_full().bg(col))
        });
        let tagged = self.tags.tagged(&self.filter);
        let rows = tagged.iter().enumerate().map(|(i, path)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let parent = path
                .parent()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            let is_dir = path.is_dir();
            let target = path.clone();
            let dots = self
                .tags
                .of(path)
                .iter()
                .map(|c| div().size(px(8.0)).rounded_full().bg(hex(color_of(c))));
            div()
                .id(("tagged", i))
                .h(px(32.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .cursor_pointer()
                .hover(|s| s.bg(white_alpha(0.04)))
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.emit(if is_dir {
                        ToolEvent::Open(target.clone())
                    } else {
                        ToolEvent::Reveal(target.clone())
                    })
                }))
                .child(file_icon(&name, is_dir, false, &p))
                .child(
                    div()
                        .flex_none()
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
                        .child(parent),
                )
                .child(div().flex().gap(px(3.0)).children(dots))
        });
        let count = tagged.len();
        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(header("Tags", subject, &p))
            .child(div().flex().flex_wrap().gap(px(8.0)).children(swatches))
            .child(
                div()
                    .mt(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_strong)
                            .child(format!("Tagged {}", self.filter)),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(p.text_muted)
                            .child(format!("{count}")),
                    )
                    .child(div().flex_1())
                    .children(filters),
            )
            .child(if count == 0 {
                list_box(&p).child(empty(
                    "tag",
                    format!("Nothing is tagged {} yet.", self.filter),
                    &p,
                ))
            } else {
                list_box(&p).child(
                    div()
                        .id("tagged-list")
                        .flex_1()
                        .overflow_y_scroll()
                        .children(rows),
                )
            })
    }
}
