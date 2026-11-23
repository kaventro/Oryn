use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::icons::icon;
use crate::theme::{Palette, hex, white_alpha, with_alpha};

pub mod checksum;
pub mod compare;
pub mod disk;
pub mod duplicates;
pub mod git;
pub mod replace;
pub mod search;
pub mod sftp;
pub mod tags;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Search,
    Replace,
    Git,
    Compare,
    Duplicates,
    Checksum,
    Disk,
    Tags,
    Sftp,
}

impl ToolKind {
    pub const ALL: [ToolKind; 9] = [
        ToolKind::Search,
        ToolKind::Replace,
        ToolKind::Git,
        ToolKind::Compare,
        ToolKind::Duplicates,
        ToolKind::Checksum,
        ToolKind::Disk,
        ToolKind::Tags,
        ToolKind::Sftp,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Search => "Search",
            ToolKind::Replace => "Find and Replace",
            ToolKind::Git => "Git",
            ToolKind::Compare => "Compare and Sync",
            ToolKind::Duplicates => "Duplicates",
            ToolKind::Checksum => "Checksums",
            ToolKind::Disk => "Disk Space",
            ToolKind::Tags => "Tags",
            ToolKind::Sftp => "SFTP",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ToolKind::Search => "search",
            ToolKind::Replace => "replace",
            ToolKind::Git => "git-branch",
            ToolKind::Compare => "git-compare",
            ToolKind::Duplicates => "files",
            ToolKind::Checksum => "hash",
            ToolKind::Disk => "chart-pie",
            ToolKind::Tags => "tag",
            ToolKind::Sftp => "server",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolContext {
    pub dir: PathBuf,
    pub other: PathBuf,
    pub targets: Vec<PathBuf>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ToolEvent {
    Reveal(PathBuf),
    Open(PathBuf),
    Changed,
}

pub fn run_bg<T: 'static, R: Send + 'static>(
    cx: &mut Context<T>,
    work: impl FnOnce() -> R + Send + 'static,
    done: impl FnOnce(&mut T, R, &mut Context<T>) + 'static,
) -> Task<()> {
    let task = cx.background_spawn(async move { work() });
    cx.spawn(async move |this, cx| {
        let ticker = {
            let this = this.clone();
            let cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                    if this.update(&mut cx.clone(), |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        };
        if let futures::future::Either::Left((r, _)) =
            futures::future::select(Box::pin(task), Box::pin(ticker)).await
        {
            let _ = this.update(cx, |t, cx| {
                done(t, r, cx);
                cx.notify();
            });
        }
    })
}

pub fn input<T: 'static>(
    placeholder: &'static str,
    window: &mut Window,
    cx: &mut Context<T>,
    on_enter: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> Entity<InputState> {
    let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    cx.subscribe_in(&state, window, move |this, _, ev: &InputEvent, w, cx| {
        if matches!(ev, InputEvent::PressEnter { .. }) {
            on_enter(this, w, cx);
        }
    })
    .detach();
    state
}

pub fn value(state: &Entity<InputState>, cx: &App) -> String {
    state.read(cx).value().to_string()
}

pub fn field(state: &Entity<InputState>, p: &Palette) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .h(px(30.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .bg(white_alpha(0.07))
        .border_1()
        .border_color(white_alpha(0.14))
        .child(
            Input::new(state)
                .appearance(false)
                .text_size(px(12.5))
                .text_color(p.text_strong),
        )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Danger,
    Plain,
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: Tone,
    p: &Palette,
) -> Stateful<Div> {
    let label = label.into();
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .flex()
        .items_center()
        .gap(px(6.0))
        .flex_none()
        .px(px(12.0))
        .h(px(28.0))
        .rounded(px(6.0))
        .text_size(px(12.5))
        .cursor_pointer();
    match tone {
        Tone::Primary => base
            .bg(p.accent)
            .text_color(p.on_accent)
            .font_weight(FontWeight::SEMIBOLD)
            .hover({
                let h = p.accent_hover;
                move |s| s.bg(h)
            }),
        Tone::Danger => base
            .bg(hex(0xff453a))
            .text_color(hex(0xffffff))
            .font_weight(FontWeight::SEMIBOLD)
            .hover(|s| s.bg(hex(0xe0382e))),
        Tone::Plain => base
            .bg(white_alpha(0.08))
            .border_1()
            .border_color(white_alpha(0.1))
            .text_color(hex(0xd1d1d6))
            .hover(|s| s.bg(white_alpha(0.14))),
    }
    .child(label)
}

pub fn chip(id: impl Into<ElementId>, label: &'static str, on: bool, p: &Palette) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(if on { Toggled::True } else { Toggled::False })
        .flex_none()
        .px(px(9.0))
        .h(px(24.0))
        .flex()
        .items_center()
        .rounded(px(12.0))
        .text_size(px(11.5))
        .cursor_pointer()
        .border_1()
        .when(on, |d| {
            d.bg(with_alpha(p.accent, 0.2))
                .border_color(with_alpha(p.accent, 0.6))
                .text_color(p.text_strong)
        })
        .when(!on, |d| {
            d.border_color(white_alpha(0.12))
                .text_color(p.text_muted)
                .hover(|s| s.bg(white_alpha(0.06)))
        })
        .child(label)
}

pub fn header(title: &'static str, subtitle: impl Into<SharedString>, p: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_strong)
                .child(title),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(p.text_muted)
                .truncate()
                .child(subtitle.into()),
        )
}

pub fn status_line(text: impl Into<SharedString>, error: bool, p: &Palette) -> Div {
    div()
        .text_size(px(12.0))
        .text_color(if error { hex(0xff6961) } else { p.text_muted })
        .child(text.into())
}

pub fn empty(icon_name: &'static str, text: impl Into<SharedString>, p: &Palette) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .text_size(px(12.5))
        .text_color(p.text_muted)
        .child(icon(icon_name, 28.0, p.text_dim))
        .child(text.into())
}

pub fn list_box(p: &Palette) -> Div {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .rounded(px(8.0))
        .bg(p.card)
        .border_1()
        .border_color(white_alpha(0.08))
        .overflow_hidden()
}

pub fn rel_to(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .map(|r| r.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}
