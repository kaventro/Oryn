use gpui_kit::*;
use oryn_core::media::MediaInfo;

use crate::theme::{Palette, white_alpha};

pub enum Load<T> {
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Load::Ready(v) => Some(v),
            _ => None,
        }
    }

    pub fn from_result(r: anyhow::Result<T>) -> Self {
        match r {
            Ok(v) => Load::Ready(v),
            Err(e) => Load::Failed(e.to_string()),
        }
    }
}

pub fn facts_row(info: &MediaInfo, skip: &[&str], p: &Palette) -> Div {
    div()
        .flex()
        .flex_wrap()
        .justify_center()
        .gap(px(8.0))
        .children(
            info.facts()
                .into_iter()
                .filter(|(label, _)| !skip.contains(label))
                .map(|(label, value)| {
                    div()
                        .flex()
                        .gap(px(6.0))
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .bg(white_alpha(0.05))
                        .border_1()
                        .border_color(white_alpha(0.06))
                        .text_size(px(12.0))
                        .child(div().text_color(p.text_muted).child(label))
                        .child(div().text_color(p.text_strong).child(value))
                }),
        )
}
