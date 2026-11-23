use std::ops::Range;
use std::path::PathBuf;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::format::{fmt_size, time_text};
use oryn_core::preview::{self, Block, Kind, Lang, Span, Tok};
use oryn_core::services::fs_office::{OfficeDoc, read_office};

use crate::actions::*;
use crate::icons::{file_icon, icon};
use crate::theme::{Palette, black_alpha, hex, mono_font, palette, white_alpha, with_alpha};

const MAX_TEXT: usize = 2 * 1024 * 1024;

pub enum QuickEvent {
    Close,
    Step(isize),
}

enum Body {
    Image,
    Lines {
        lines: Vec<SharedString>,
        lang: Option<Lang>,
        truncated: bool,
    },
    Markdown(Vec<Block>),
    Office(OfficeDoc),
    Media,
    Error(String),
}

pub struct QuickView {
    path: PathBuf,
    kind: Kind,
    size: u64,
    modified: String,
    body: Body,
    hex: bool,
    highlights: Vec<Vec<(Range<usize>, HighlightStyle)>>,
    pub focus: FocusHandle,
}

impl EventEmitter<QuickEvent> for QuickView {}

fn tok_color(t: Tok) -> Option<Hsla> {
    Some(match t {
        Tok::Keyword => hex(0xff7ab2),
        Tok::String => hex(0xff8170),
        Tok::Number => hex(0xd9c97c),
        Tok::Comment => hex(0x7f8c98),
        Tok::Type => hex(0x5dd8ff),
        Tok::Punct => hex(0xa9b1ba),
        Tok::Plain => return None,
    })
}

pub fn open_default(path: &std::path::Path) {
    crate::opener::open(path);
}

impl QuickView {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            path: PathBuf::new(),
            kind: Kind::Binary,
            size: 0,
            modified: String::new(),
            body: Body::Media,
            hex: false,
            highlights: Vec::new(),
            focus: cx.focus_handle(),
        };
        view.load(path);
        view
    }

    pub fn load(&mut self, path: PathBuf) {
        let meta = std::fs::metadata(&path).ok();
        self.size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        self.modified = meta
            .and_then(|m| m.modified().ok())
            .map(time_text)
            .unwrap_or_default();
        self.kind = preview::kind_of(&path);
        self.highlights.clear();
        let text = |lang: Option<Lang>| match preview::read_text(&path, MAX_TEXT) {
            Ok((t, truncated)) => Body::Lines {
                lines: t
                    .lines()
                    .map(|l| SharedString::from(l.replace('\t', "    ")))
                    .collect(),
                lang,
                truncated,
            },
            Err(e) => Body::Error(e.to_string()),
        };
        self.body = match self.kind {
            Kind::Image => Body::Image,
            Kind::Code(lang) => text(Some(lang)),
            Kind::Text => text(None),
            Kind::Markdown => match preview::read_text(&path, MAX_TEXT) {
                Ok((t, _)) => Body::Markdown(preview::markdown(&t)),
                Err(e) => Body::Error(e.to_string()),
            },
            Kind::Office => match read_office(&path.to_string_lossy()) {
                Ok(doc) => Body::Office(doc),
                Err(e) => Body::Error(e.to_string()),
            },
            Kind::Audio | Kind::Video | Kind::Pdf | Kind::Binary => Body::Media,
        };
        if let Body::Lines {
            lines,
            lang: Some(lang),
            ..
        } = &self.body
        {
            let mut block = false;
            self.highlights = lines
                .iter()
                .map(|l| {
                    let mut at = 0;
                    preview::highlight(l, *lang, &mut block)
                        .into_iter()
                        .filter_map(|(t, s)| {
                            let r = at..at + s.len();
                            at = r.end;
                            tok_color(t).map(|c| {
                                (
                                    r,
                                    HighlightStyle {
                                        color: Some(c),
                                        ..Default::default()
                                    },
                                )
                            })
                        })
                        .collect()
                })
                .collect();
        }
        self.hex = matches!(self.body, Body::Media) && self.kind == Kind::Binary;
        self.path = path;
    }

    fn kind_label(&self) -> String {
        let ext = preview::extension(&self.path).to_uppercase();
        let base = match self.kind {
            Kind::Image => "Image",
            Kind::Markdown => "Markdown",
            Kind::Code(_) => "Source code",
            Kind::Text => "Text",
            Kind::Office => "Document",
            Kind::Audio => "Audio",
            Kind::Video => "Video",
            Kind::Pdf => "PDF document",
            Kind::Binary => "File",
        };
        if ext.is_empty() {
            base.to_string()
        } else {
            format!("{base} \u{00b7} {ext}")
        }
    }

    fn render_lines(&self, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let Body::Lines {
            lines, truncated, ..
        } = &self.body
        else {
            return div().into_any_element();
        };
        let count = lines.len();
        let digits = count.max(1).to_string().len() as f32;
        let truncated = *truncated;
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                uniform_list(
                    "qv-lines",
                    count,
                    cx.processor(move |this, range: Range<usize>, _, cx| {
                        let p = palette(cx);
                        let Body::Lines { lines, .. } = &this.body else {
                            return Vec::new();
                        };
                        range
                            .map(|i| {
                                let text = lines[i].clone();
                                let styled = match this.highlights.get(i) {
                                    Some(h) if !h.is_empty() => {
                                        StyledText::new(text).with_highlights(h.clone())
                                    }
                                    _ => StyledText::new(text),
                                };
                                div()
                                    .id(("qv-line", i))
                                    .flex()
                                    .h(px(19.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_none()
                                            .w(px(digits * 8.0 + 22.0))
                                            .pr(px(14.0))
                                            .text_right()
                                            .text_color(p.text_dim)
                                            .child((i + 1).to_string()),
                                    )
                                    .child(
                                        div().whitespace_nowrap().text_color(p.text).child(styled),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
                )
                .flex_1()
                .py(px(8.0))
                .font_family(mono_font())
                .text_size(px(12.5)),
            )
            .when(truncated, |d| {
                d.child(
                    div()
                        .px(px(14.0))
                        .py(px(6.0))
                        .text_size(px(11.5))
                        .text_color(p.text_muted)
                        .border_t_1()
                        .border_color(white_alpha(0.06))
                        .child("Showing the first 2 MB."),
                )
            })
            .into_any_element()
    }

    fn render_markdown(&self, blocks: &[Block], p: &Palette) -> AnyElement {
        let rich = |text: &str, base: Hsla| {
            let mut s = String::new();
            let mut hl = Vec::new();
            for (span, part) in preview::inline(text) {
                let start = s.len();
                s.push_str(&part);
                let style = match span {
                    Span::Plain => continue,
                    Span::Bold => HighlightStyle {
                        font_weight: Some(FontWeight::BOLD),
                        color: Some(p.text_strong),
                        ..Default::default()
                    },
                    Span::Italic => HighlightStyle {
                        font_style: Some(FontStyle::Italic),
                        ..Default::default()
                    },
                    Span::Code => HighlightStyle {
                        color: Some(hex(0xff8170)),
                        background_color: Some(white_alpha(0.08)),
                        ..Default::default()
                    },
                    Span::Link => HighlightStyle {
                        color: Some(p.accent),
                        underline: Some(UnderlineStyle {
                            thickness: px(1.0),
                            color: Some(with_alpha(p.accent, 0.5)),
                            wavy: false,
                        }),
                        ..Default::default()
                    },
                };
                hl.push((start..s.len(), style));
            }
            div()
                .text_color(base)
                .child(StyledText::new(s).with_highlights(hl))
        };
        div()
            .id("qv-md")
            .size_full()
            .overflow_y_scroll()
            .px(px(36.0))
            .py(px(24.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .text_size(px(14.0))
            .line_height(px(22.0))
            .children(blocks.iter().take(3000).map(|b| {
                match b {
                    Block::Heading(level, text) => {
                        let size = match level {
                            1 => 26.0,
                            2 => 21.0,
                            3 => 17.0,
                            _ => 15.0,
                        };
                        div()
                            .mt(px(if *level <= 2 { 10.0 } else { 4.0 }))
                            .text_size(px(size))
                            .line_height(px(size * 1.3))
                            .font_weight(FontWeight::BOLD)
                            .when(*level <= 2, |d| {
                                d.pb(px(6.0)).border_b_1().border_color(white_alpha(0.08))
                            })
                            .child(rich(text, p.text_strong))
                            .into_any_element()
                    }
                    Block::Paragraph(text) => rich(text, p.text).into_any_element(),
                    Block::Bullet(depth, text) => div()
                        .flex()
                        .gap(px(8.0))
                        .pl(px(*depth as f32 * 20.0 + 4.0))
                        .child(div().text_color(p.text_muted).child("\u{2022}"))
                        .child(rich(text, p.text))
                        .into_any_element(),
                    Block::Numbered(n, text) => div()
                        .flex()
                        .gap(px(8.0))
                        .pl(px(4.0))
                        .child(div().text_color(p.text_muted).child(format!("{n}.")))
                        .child(rich(text, p.text))
                        .into_any_element(),
                    Block::Quote(text) => div()
                        .pl(px(14.0))
                        .border_l_2()
                        .border_color(with_alpha(p.accent, 0.6))
                        .child(rich(text, p.text_secondary))
                        .into_any_element(),
                    Block::Code(code) => div()
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .bg(black_alpha(0.3))
                        .border_1()
                        .border_color(white_alpha(0.06))
                        .font_family(mono_font())
                        .text_size(px(12.5))
                        .line_height(px(19.0))
                        .text_color(p.text)
                        .child(code.clone())
                        .into_any_element(),
                    Block::Rule => div()
                        .h(px(1.0))
                        .my(px(6.0))
                        .bg(white_alpha(0.1))
                        .into_any_element(),
                }
            }))
            .into_any_element()
    }

    fn render_office(&self, doc: &OfficeDoc, p: &Palette) -> AnyElement {
        if !doc.sheets.is_empty() {
            return div()
                .id("qv-sheets")
                .size_full()
                .overflow_scroll()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(18.0))
                .children(doc.sheets.iter().map(|sheet| {
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .mb(px(6.0))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_strong)
                                .child(sheet.name.clone()),
                        )
                        .children(sheet.rows.iter().take(500).enumerate().map(|(ri, row)| {
                            div()
                                .flex()
                                .when(ri == 0, |d| {
                                    d.font_weight(FontWeight::SEMIBOLD).bg(white_alpha(0.05))
                                })
                                .children(row.iter().map(|cell| {
                                    div()
                                        .w(px(140.0))
                                        .flex_none()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .border_1()
                                        .border_color(white_alpha(0.06))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(p.text)
                                        .child(cell.clone())
                                }))
                        }))
                }))
                .into_any_element();
        }
        div()
            .id("qv-doc")
            .size_full()
            .overflow_y_scroll()
            .px(px(40.0))
            .py(px(28.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .text_size(px(14.0))
            .line_height(px(22.0))
            .text_color(p.text)
            .children(
                doc.paragraphs
                    .iter()
                    .take(3000)
                    .map(|para| div().child(para.clone())),
            )
            .into_any_element()
    }

    fn render_hex(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = (self.size.min(256 * 1024 * 1024)).div_ceil(16) as usize;
        let path = self.path.clone();
        uniform_list(
            "qv-hex",
            rows,
            cx.processor(move |_, range: Range<usize>, _, cx| {
                let p = palette(cx);
                let start = range.start as u64 * 16;
                let len = range.len() * 16;
                preview::hex_dump(&path, start, len)
                    .unwrap_or_default()
                    .into_iter()
                    .enumerate()
                    .map(|(i, line)| {
                        div()
                            .id(("hex", range.start + i))
                            .flex()
                            .gap(px(18.0))
                            .h(px(19.0))
                            .items_center()
                            .child(
                                div()
                                    .w(px(80.0))
                                    .text_color(p.text_dim)
                                    .child(format!("{:08x}", line.offset)),
                            )
                            .child(div().w(px(400.0)).text_color(p.text).child(line.hex))
                            .child(div().text_color(p.text_muted).child(line.ascii))
                            .into_any_element()
                    })
                    .collect()
            }),
        )
        .size_full()
        .px(px(16.0))
        .py(px(8.0))
        .font_family(mono_font())
        .text_size(px(12.5))
        .into_any_element()
    }

    fn render_media(&self, p: &Palette) -> AnyElement {
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let hint = match self.kind {
            Kind::Audio => "Audio plays in your default player.",
            Kind::Video => "Video plays in your default player.",
            Kind::Pdf => "PDFs open in your default viewer.",
            _ => "No preview for this kind of file. Try the Hex view.",
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(14.0))
            .child(file_icon(&name, false, false, p).size(px(96.0)))
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_strong)
                    .child(name),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(p.text_muted)
                    .child(format!(
                        "{} \u{00b7} {}",
                        self.kind_label(),
                        fmt_size(Some(self.size))
                    )),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(p.text_muted)
                    .child(hint),
            )
            .into_any_element()
    }
}

impl Focusable for QuickView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for QuickView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let can_hex = !matches!(self.body, Body::Image) && self.size > 0;
        let body = if self.hex && can_hex {
            self.render_hex(cx)
        } else {
            match &self.body {
                Body::Image => div()
                    .size_full()
                    .p(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        img(self.path.clone())
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .with_fallback({
                                let c = p.text_muted;
                                move || {
                                    div()
                                        .text_color(c)
                                        .child("This image can't be shown.")
                                        .into_any_element()
                                }
                            }),
                    )
                    .into_any_element(),
                Body::Lines { .. } => self.render_lines(&p, cx),
                Body::Markdown(blocks) => self.render_markdown(blocks, &p),
                Body::Office(doc) => self.render_office(doc, &p),
                Body::Media => self.render_media(&p),
                Body::Error(e) => div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(hex(0xff6961))
                    .child(e.clone())
                    .into_any_element(),
            }
        };
        let tab = |id: &'static str, label: &'static str, on: bool| {
            div()
                .id(id)
                .px(px(10.0))
                .py(px(3.0))
                .rounded(px(5.0))
                .text_size(px(12.0))
                .cursor_pointer()
                .text_color(if on { p.text_strong } else { p.text_muted })
                .when(on, |d| d.bg(white_alpha(0.1)))
                .when(!on, |d| d.hover(|s| s.text_color(p.text_strong)))
                .child(label)
        };
        let path = self.path.clone();
        div()
            .id("quick-view")
            .key_context("QuickView")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CancelPrompt, _, cx| cx.emit(QuickEvent::Close)))
            .on_action(cx.listener(|_, _: &QuickLook, _, cx| cx.emit(QuickEvent::Close)))
            .on_action(cx.listener(|_, _: &MoveUp, _, cx| cx.emit(QuickEvent::Step(-1))))
            .on_action(cx.listener(|_, _: &MoveDown, _, cx| cx.emit(QuickEvent::Step(1))))
            .on_action(cx.listener(|this, _: &ToggleHex, _, cx| {
                this.hex = !this.hex;
                cx.notify();
            }))
            .size_full()
            .flex()
            .flex_col()
            .rounded(px(14.0))
            .bg(hex(0x1c1c20).opacity(0.98))
            .border_1()
            .border_color(white_alpha(0.12))
            .overflow_hidden()
            .shadow(vec![BoxShadow {
                color: black_alpha(0.75),
                offset: point(px(0.0), px(28.0)),
                blur_radius: px(70.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .h(px(48.0))
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(white_alpha(0.08))
                    .child(file_icon(&name, false, false, &p))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().text_size(px(13.0)).font_weight(FontWeight::SEMIBOLD).text_color(p.text_strong).child(name))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(p.text_muted)
                                    .child(format!("{} \u{00b7} {} \u{00b7} {}", self.kind_label(), fmt_size(Some(self.size)), self.modified)),
                            ),
                    )
                    .when(can_hex, |d| {
                        d.child(
                            div()
                                .flex()
                                .p(px(2.0))
                                .rounded(px(7.0))
                                .bg(white_alpha(0.05))
                                .child(tab("qv-preview", "Preview", !self.hex).on_click(cx.listener(|this, _, _, cx| {
                                    this.hex = false;
                                    cx.notify();
                                })))
                                .child(tab("qv-hex", "Hex", self.hex).on_click(cx.listener(|this, _, _, cx| {
                                    this.hex = true;
                                    cx.notify();
                                }))),
                        )
                    })
                    .child(
                        div()
                            .id("qv-open")
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(10.0))
                            .h(px(26.0))
                            .rounded(px(6.0))
                            .text_size(px(12.0))
                            .bg(p.accent)
                            .text_color(p.on_accent)
                            .cursor_pointer()
                            .hover(|s| s.bg(p.accent_hover))
                            .on_click(move |_, _, _| open_default(&path))
                            .child("Open"),
                    )
                    .child(
                        div()
                            .id("qv-close")
                            .size(px(26.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(white_alpha(0.08)))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(QuickEvent::Close)))
                            .child(icon("x", 14.0, p.text_muted)),
                    ),
            )
            .child(div().flex_1().min_h_0().child(body))
            .child(
                div()
                    .flex_none()
                    .px(px(14.0))
                    .py(px(5.0))
                    .text_size(px(11.0))
                    .text_color(p.text_dim)
                    .border_t_1()
                    .border_color(white_alpha(0.06))
                    .child(if can_hex {
                        "\u{2191} \u{2193} previous or next file \u{00b7} Tab switches to Hex \u{00b7} Esc closes"
                    } else {
                        "\u{2191} \u{2193} previous or next file \u{00b7} Esc closes"
                    }),
            )
    }
}
