use std::io::{Read, Write};
use std::path::Path;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, Rgb};
use futures::StreamExt;
use futures::channel::mpsc;
use gpui_kit::*;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::theme::{Palette, hex, mono_font, palette};

type GridCell = (char, Hsla, Option<Hsla>, bool);

const FONT_SIZE: f32 = 12.5;
const LINE_HEIGHT: f32 = 18.0;
const CELL_WIDTH: f32 = 7.6;

#[derive(Clone, Copy)]
struct Size {
    cols: usize,
    rows: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

#[derive(Clone)]
struct Listener;

impl EventListener for Listener {
    fn send_event(&self, _: Event) {}
}

pub struct TerminalView {
    term: Term<Listener>,
    parser: Processor,
    writer: Option<Box<dyn Write + Send>>,
    master: Option<Box<dyn MasterPty + Send>>,
    size: Size,
    pub focus: FocusHandle,
    exited: bool,
    cwd: std::path::PathBuf,
}

pub fn cd_command(dir: &Path, windows: bool) -> String {
    let d = dir.to_string_lossy();
    if windows {
        format!("Set-Location -LiteralPath '{}'\r", d.replace('\'', "''"))
    } else {
        format!("cd -- '{}'\r", d.replace('\'', "'\\''"))
    }
}

impl TerminalView {
    pub fn new(cwd: &Path, cx: &mut Context<Self>) -> Self {
        let size = Size {
            cols: 120,
            rows: 12,
        };
        let term = Term::new(Config::default(), &size, Listener);
        let mut view = Self {
            term,
            parser: Processor::new(),
            writer: None,
            master: None,
            size,
            focus: cx.focus_handle(),
            exited: false,
            cwd: cwd.to_path_buf(),
        };
        if let Err(err) = view.spawn(cwd, cx) {
            let msg = format!("Couldn't start a shell: {err}\r\n");
            view.parser.advance(&mut view.term, msg.as_bytes());
        }
        view
    }

    fn spawn(&mut self, cwd: &Path, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: self.size.rows as u16,
            cols: self.size.cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "powershell.exe".into()
            } else {
                "/bin/sh".into()
            }
        });
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        let _child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        self.writer = Some(pair.master.take_writer()?);
        self.master = Some(pair.master);

        let (tx, mut rx) = mpsc::unbounded::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.unbounded_send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        cx.spawn(async move |this, cx| {
            while let Some(mut chunk) = rx.next().await {
                while let Ok(more) = rx.try_recv() {
                    chunk.extend(more);
                }
                if this
                    .update(cx, |view, cx| {
                        view.parser.advance(&mut view.term, &chunk);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            let _ = this.update(cx, |view, cx| {
                view.exited = true;
                cx.notify();
            });
        })
        .detach();
        Ok(())
    }

    pub fn has_exited(&self) -> bool {
        self.exited
    }

    pub fn follow(&mut self, dir: &Path) {
        if self.exited || dir == self.cwd {
            return;
        }
        self.cwd = dir.to_path_buf();
        let cmd = cd_command(dir, cfg!(windows));
        self.write(cmd.as_bytes());
    }

    fn write(&mut self, bytes: &[u8]) {
        if let Some(w) = self.writer.as_mut() {
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }

    fn resize(&mut self, width: f32, height: f32) {
        let cols = ((width - 24.0) / CELL_WIDTH).floor().max(20.0) as usize;
        let rows = ((height - 16.0) / LINE_HEIGHT).floor().max(3.0) as usize;
        if cols == self.size.cols && rows == self.size.rows {
            return;
        }
        self.size = Size { cols, rows };
        self.term.resize(self.size);
        if let Some(m) = self.master.as_ref() {
            let _ = m.resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        if ks.modifiers.control && ks.key == "`" {
            return;
        }
        let paste = ks.key == "v"
            && (ks.modifiers.platform || (ks.modifiers.control && ks.modifiers.shift));
        if paste {
            if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                self.write(text.replace("\r\n", "\r").replace('\n', "\r").as_bytes());
            }
            cx.stop_propagation();
            return;
        }
        let bytes: Option<Vec<u8>> = match ks.key.as_str() {
            "enter" => Some(b"\r".to_vec()),
            "backspace" => Some(vec![0x7f]),
            "tab" => Some(b"\t".to_vec()),
            "escape" => Some(vec![0x1b]),
            "up" => Some(b"\x1b[A".to_vec()),
            "down" => Some(b"\x1b[B".to_vec()),
            "right" => Some(b"\x1b[C".to_vec()),
            "left" => Some(b"\x1b[D".to_vec()),
            "home" => Some(b"\x1b[H".to_vec()),
            "end" => Some(b"\x1b[F".to_vec()),
            "delete" => Some(b"\x1b[3~".to_vec()),
            k if ks.modifiers.control && k.len() == 1 => {
                let c = k.as_bytes()[0].to_ascii_lowercase();
                c.is_ascii_lowercase().then(|| vec![c - b'a' + 1])
            }
            _ => ks.key_char.as_ref().map(|s| s.as_bytes().to_vec()),
        };
        if let Some(b) = bytes {
            self.write(&b);
            cx.stop_propagation();
        }
    }

    fn lines(&self, p: &Palette) -> Vec<StyledText> {
        let content = self.term.renderable_content();
        let cursor = content.cursor.point;
        let cols = self.size.cols;
        let mut grid: Vec<Vec<GridCell>> =
            vec![vec![(' ', p.text, None, false); cols]; self.size.rows];
        for cell in content.display_iter {
            let Point {
                line: Line(l),
                column: Column(c),
            } = cell.point;
            if l < 0 || l as usize >= grid.len() || c >= cols {
                continue;
            }
            let flags = cell.cell.flags;
            if flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let (mut fg, mut bg) = (
                color(cell.cell.fg, p, true),
                (cell.cell.bg != Color::Named(NamedColor::Background))
                    .then(|| color(cell.cell.bg, p, false)),
            );
            if flags.contains(Flags::INVERSE) {
                let old_fg = fg;
                fg = bg.unwrap_or(p.terminal_bg);
                bg = Some(old_fg);
            }
            grid[l as usize][c] = (cell.cell.c, fg, bg, flags.contains(Flags::BOLD));
        }
        let cl = cursor.line.0;
        if !self.exited && cl >= 0 && (cl as usize) < grid.len() && cursor.column.0 < cols {
            let slot = &mut grid[cl as usize][cursor.column.0];
            slot.1 = p.terminal_bg;
            slot.2 = Some(p.accent);
        }
        grid.into_iter()
            .map(|row| {
                let mut text = String::new();
                let mut runs: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
                for (ch, fg, bg, bold) in row {
                    let start = text.len();
                    text.push(if ch == '\0' { ' ' } else { ch });
                    let style = HighlightStyle {
                        color: Some(fg),
                        background_color: bg,
                        font_weight: bold.then_some(FontWeight::BOLD),
                        ..Default::default()
                    };
                    match runs.last_mut() {
                        Some((r, s)) if *s == style && r.end == start => r.end = text.len(),
                        _ => runs.push((start..text.len(), style)),
                    }
                }
                StyledText::new(text).with_highlights(runs)
            })
            .collect()
    }
}

fn color(c: Color, p: &Palette, fg: bool) -> Hsla {
    match c {
        Color::Spec(Rgb { r, g, b }) => hex(((r as u32) << 16) | ((g as u32) << 8) | b as u32),
        Color::Indexed(i) => indexed(i),
        Color::Named(n) => match n {
            NamedColor::Foreground | NamedColor::BrightForeground => p.text,
            NamedColor::Background => p.terminal_bg,
            NamedColor::DimForeground => p.text_muted,
            NamedColor::Cursor => p.accent,
            other => {
                let i = other as usize;
                if i < 16 {
                    indexed(i as u8)
                } else if fg {
                    p.text
                } else {
                    p.terminal_bg
                }
            }
        },
    }
}

fn indexed(i: u8) -> Hsla {
    const BASE: [u32; 16] = [
        0x1c1c1e, 0xff453a, 0x34c759, 0xffd60a, 0x0a84ff, 0xbf5af2, 0x30b0c7, 0xd1d1d6, 0x636366,
        0xff6961, 0x5ee07a, 0xffe45c, 0x409cff, 0xda8fff, 0x70d7ff, 0xf5f5f7,
    ];
    match i {
        0..=15 => hex(BASE[i as usize]),
        16..=231 => {
            let i = i - 16;
            let step = |v: u8| if v == 0 { 0 } else { 55 + v as u32 * 40 };
            hex((step(i / 36) << 16) | (step((i / 6) % 6) << 8) | step(i % 6))
        }
        _ => {
            let v = 8 + (i as u32 - 232) * 10;
            hex((v << 16) | (v << 8) | v)
        }
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let vw = window.viewport_size().width.as_f32() - 220.0;
        self.resize(vw, 250.0);
        let lines = self.lines(&p);
        div()
            .id("terminal")
            .key_context("Terminal")
            .track_focus(&self.focus)
            .role(Role::Terminal)
            .aria_label("Terminal")
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)))
            .size_full()
            .px(px(12.0))
            .py(px(8.0))
            .bg(p.terminal_bg)
            .font_family(mono_font())
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_HEIGHT))
            .text_color(p.text)
            .overflow_hidden()
            .children(
                lines
                    .into_iter()
                    .map(|l| div().h(px(LINE_HEIGHT)).whitespace_nowrap().child(l)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::cd_command;
    use std::path::Path;

    #[test]
    fn quotes_cd_for_each_shell() {
        assert_eq!(
            cd_command(Path::new("/tmp/it's"), false),
            "cd -- '/tmp/it'\\''s'\r"
        );
        assert_eq!(
            cd_command(Path::new("C:\\it's"), true),
            "Set-Location -LiteralPath 'C:\\it''s'\r"
        );
    }
}
