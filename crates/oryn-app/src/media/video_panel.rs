use std::path::PathBuf;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::media::MediaInfo;

use super::MediaServices;
use super::facts::{Load, facts_row};
use crate::icons::icon;
use crate::theme::{black_alpha, palette, white_alpha, with_alpha};

const FRAME_SIZE: u32 = 960;

pub struct VideoPanel {
    path: PathBuf,
    services: MediaServices,
    info: Load<MediaInfo>,
    frame: Load<Option<PathBuf>>,
    _tasks: [Task<()>; 2],
}

impl VideoPanel {
    pub fn new(path: PathBuf, services: MediaServices, cx: &mut Context<Self>) -> Self {
        let probe = {
            let probe = services.probe.clone();
            let target = path.clone();
            let task = cx.background_spawn(async move { probe.probe(&target) });
            cx.spawn(async move |this, cx| {
                let info = task.await;
                let _ = this.update(cx, |panel, cx| {
                    panel.info = Load::from_result(info);
                    cx.notify();
                });
            })
        };
        let frame = {
            let frames = services.frames.clone();
            let target = path.clone();
            let task = cx.background_spawn(async move { frames.frame(&target, FRAME_SIZE) });
            cx.spawn(async move |this, cx| {
                let frame = task.await;
                let _ = this.update(cx, |panel, cx| {
                    panel.frame = Load::Ready(frame);
                    cx.notify();
                });
            })
        };
        Self {
            path,
            services,
            info: Load::Loading,
            frame: Load::Loading,
            _tasks: [probe, frame],
        }
    }

    pub fn play(&self) {
        self.services.launcher.launch(&self.path);
    }

    pub fn frame(&self) -> Option<&PathBuf> {
        self.frame.ready().and_then(|f| f.as_ref())
    }

    #[cfg(test)]
    pub fn info(&self) -> Option<&MediaInfo> {
        self.info.ready()
    }
}

impl Render for VideoPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let note = match (&self.frame, self.frame()) {
            (Load::Loading, _) => Some("Making a preview\u{2026}"),
            (_, None) => Some("No preview picture on this computer"),
            _ => None,
        };
        let picture = self.frame().map(|frame| {
            img(frame.clone())
                .size_full()
                .object_fit(ObjectFit::Contain)
        });
        let status = match &self.info {
            Load::Loading => Some("Reading the file\u{2026}".to_string()),
            Load::Failed(e) => Some(e.clone()),
            Load::Ready(_) => None,
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .p(px(20.0))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .rounded(px(12.0))
                    .overflow_hidden()
                    .bg(black_alpha(0.4))
                    .border_1()
                    .border_color(white_alpha(0.06))
                    .when_some(picture, |d, picture| d.child(picture))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .id("media-play")
                                    .debug_selector(|| "media-play".into())
                                    .size(px(68.0))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(black_alpha(0.55))
                                    .border_1()
                                    .border_color(white_alpha(0.25))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(with_alpha(p.accent, 0.9)))
                                    .on_click(cx.listener(|this, _, _, _| this.play()))
                                    .child(icon("play", 28.0, p.text_strong)),
                            )
                            .when_some(note, |d, note| {
                                d.child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.text_muted)
                                        .child(note),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.0))
                    .when_some(self.info.ready(), |d, info| {
                        d.child(facts_row(info, &[], &p))
                    })
                    .when_some(status, |d, s| {
                        d.child(div().text_size(px(12.5)).text_color(p.text_muted).child(s))
                    })
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.text_dim)
                            .child("Play opens the video in your default player."),
                    ),
            )
    }
}
