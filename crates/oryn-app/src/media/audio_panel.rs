use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::media::{MediaInfo, Timeline, clock};

use super::facts::{Load, facts_row};
use super::seek_bar::SeekBar;
use super::{AudioPlayer, MediaServices};
use crate::icons::icon;
use crate::theme::{hex, mono_font, palette, with_alpha};

const TICK: Duration = Duration::from_millis(200);
const STEP: Duration = Duration::from_secs(5);

pub struct AudioPanel {
    path: PathBuf,
    services: MediaServices,
    info: Load<MediaInfo>,
    player: Option<Box<dyn AudioPlayer>>,
    position: Duration,
    error: Option<String>,
    ticker: Option<Task<()>>,
    _probe: Task<()>,
}

impl AudioPanel {
    pub fn new(path: PathBuf, services: MediaServices, cx: &mut Context<Self>) -> Self {
        let probe = services.probe.clone();
        let target = path.clone();
        let task = cx.background_spawn(async move { probe.probe(&target) });
        let _probe = cx.spawn(async move |this, cx| {
            let info = task.await;
            let _ = this.update(cx, |panel, cx| {
                panel.info = Load::from_result(info);
                cx.notify();
            });
        });
        Self {
            path,
            services,
            info: Load::Loading,
            player: None,
            position: Duration::ZERO,
            error: None,
            ticker: None,
            _probe,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.is_playing())
    }

    #[cfg(test)]
    pub fn position(&self) -> Duration {
        self.position
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn timeline(&self) -> Option<Timeline> {
        self.info
            .ready()
            .and_then(|i| i.duration)
            .map(Timeline::new)
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.is_playing() {
            self.pause(cx);
        } else {
            self.play(cx);
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        if self.player.is_none() {
            match self.services.audio.open(&self.path) {
                Ok(mut player) => {
                    if !self.position.is_zero() {
                        let _ = player.seek(self.position);
                    }
                    self.player = Some(player);
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    cx.notify();
                    return;
                }
            }
        }
        if let Some(player) = self.player.as_mut() {
            player.play();
        }
        self.start_ticker(cx);
        cx.notify();
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_mut() {
            player.pause();
            self.position = player.position();
        }
        self.ticker = None;
        cx.notify();
    }

    pub fn seek_to(&mut self, to: Duration, cx: &mut Context<Self>) {
        let to = self.timeline().map_or(to, |t| to.min(t.length()));
        self.position = to;
        if let Some(player) = self.player.as_mut()
            && let Err(e) = player.seek(to)
        {
            self.error = Some(e.to_string());
        }
        cx.notify();
    }

    pub fn seek_fraction(&mut self, fraction: f32, cx: &mut Context<Self>) {
        if let Some(t) = self.timeline() {
            self.seek_to(t.at(fraction), cx);
        }
    }

    pub fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(t) = self.timeline() else {
            return;
        };
        let to = t.step(self.position, STEP, forward);
        self.seek_to(to, cx);
    }

    fn start_ticker(&mut self, cx: &mut Context<Self>) {
        self.ticker = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                match this.update(cx, |panel, cx| panel.tick(cx)) {
                    Ok(true) => continue,
                    _ => break,
                }
            }
        }));
    }

    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(player) = self.player.as_ref() else {
            return false;
        };
        if player.is_finished() {
            self.player = None;
            self.position = Duration::ZERO;
            cx.notify();
            return false;
        }
        self.position = player.position();
        cx.notify();
        true
    }

    fn title(&self) -> (String, Option<String>) {
        let name = self
            .path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(info) = self.info.ready() else {
            return (name, None);
        };
        let by: Vec<&str> = [info.tags.artist.as_deref(), info.tags.album.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        (
            info.tags.title.clone().unwrap_or(name),
            (!by.is_empty()).then(|| by.join(" \u{00b7} ")),
        )
    }
}

impl Render for AudioPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let (title, subtitle) = self.title();
        let playing = self.is_playing();
        let length = self.timeline().map(|t| t.length());
        let fraction = self.timeline().map_or(0.0, |t| t.fraction(self.position));
        let entity = cx.entity().downgrade();
        let status = match &self.info {
            Load::Loading => Some("Reading the file\u{2026}".to_string()),
            Load::Failed(e) => Some(e.clone()),
            Load::Ready(_) => None,
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .px(px(40.0))
            .child(
                div()
                    .size(px(140.0))
                    .rounded(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(with_alpha(p.accent, 0.16))
                    .border_1()
                    .border_color(with_alpha(p.accent, 0.3))
                    .child(icon("music", 60.0, p.accent)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.0))
                    .max_w(px(560.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_strong)
                            .truncate()
                            .child(title),
                    )
                    .when_some(subtitle, |d, s| {
                        d.child(div().text_size(px(13.0)).text_color(p.text_muted).child(s))
                    }),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(560.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .id("media-play")
                            .debug_selector(|| "media-play".into())
                            .flex_none()
                            .size(px(42.0))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(p.accent)
                            .cursor_pointer()
                            .hover(|s| s.bg(p.accent_hover))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
                            .child(icon(
                                if playing { "pause" } else { "play" },
                                18.0,
                                p.on_accent,
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(px(52.0))
                            .text_right()
                            .font_family(mono_font())
                            .text_size(px(12.0))
                            .text_color(p.text)
                            .child(clock(self.position)),
                    )
                    .child(SeekBar::new("media-seek", fraction, p, move |f, _, cx| {
                        if let Some(panel) = entity.upgrade() {
                            panel.update(cx, |panel, cx| panel.seek_fraction(f, cx));
                        }
                    }))
                    .child(
                        div()
                            .flex_none()
                            .w(px(52.0))
                            .font_family(mono_font())
                            .text_size(px(12.0))
                            .text_color(p.text_muted)
                            .child(length.map(clock).unwrap_or_else(|| "--:--".into())),
                    ),
            )
            .when_some(self.info.ready(), |d, info| {
                d.child(facts_row(info, &["Duration"], &p))
            })
            .when_some(status, |d, s| {
                d.child(div().text_size(px(12.5)).text_color(p.text_muted).child(s))
            })
            .when_some(self.error.clone(), |d, e| {
                d.child(
                    div()
                        .text_size(px(12.5))
                        .text_color(hex(0xff6961))
                        .child(format!("{e}. Open plays it in your default app.")),
                )
            })
    }
}
