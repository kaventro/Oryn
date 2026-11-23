use std::path::PathBuf;

use gpui_kit::component::input::Input;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::batch::{self, BatchKind};
use oryn_core::services::fs_transfer::OverwritePolicy;
use oryn_core::ui::rename::validate_name;

use super::Workspace;
use crate::actions::*;
use crate::theme::{Palette, black_alpha, hex, white_alpha, with_alpha};

pub enum Dialog {
    Mask {
        selecting: bool,
    },
    NewFolder {
        dir: PathBuf,
    },
    NewFile {
        dir: PathBuf,
    },
    Rename {
        dir: PathBuf,
        old: String,
    },
    Transfer {
        kind: BatchKind,
        sources: Vec<PathBuf>,
        conflicts: Option<(PathBuf, Vec<String>)>,
    },
    Delete {
        sources: Vec<PathBuf>,
        permanent: bool,
    },
    GoTo,
    Info {
        path: PathBuf,
        rows: Vec<(&'static str, String)>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Confirm,
    Overwrite,
    Skip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Primary,
    Danger,
    Plain,
}

fn names(sources: &[PathBuf]) -> String {
    let shown: Vec<String> = sources
        .iter()
        .take(3)
        .map(|s| {
            s.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect();
    let more = sources.len().saturating_sub(shown.len());
    let mut text = shown.join(", ");
    if more > 0 {
        text.push_str(&format!(" and {more} more"));
    }
    text
}

fn count(n: usize) -> String {
    if n == 1 {
        "1 item".into()
    } else {
        format!("{n} items")
    }
}

impl Dialog {
    fn title(&self) -> String {
        match self {
            Dialog::Mask { selecting: true } => "Select by mask".into(),
            Dialog::Mask { selecting: false } => "Deselect by mask".into(),
            Dialog::NewFolder { .. } => "New folder".into(),
            Dialog::NewFile { .. } => "New file".into(),
            Dialog::Rename { old, .. } => format!("Rename \u{201c}{old}\u{201d}"),
            Dialog::Transfer {
                conflicts: Some((_, c)),
                ..
            } => format!(
                "{} already exist{}",
                count(c.len()),
                if c.len() == 1 { "s" } else { "" }
            ),
            Dialog::Transfer { kind, sources, .. } => format!(
                "{} {}",
                if *kind == BatchKind::Copy {
                    "Copy"
                } else {
                    "Move"
                },
                count(sources.len())
            ),
            Dialog::Delete {
                sources,
                permanent: false,
            } => format!("Move {} to the Trash?", count(sources.len())),
            Dialog::Delete {
                sources,
                permanent: true,
            } => format!("Delete {} permanently?", count(sources.len())),
            Dialog::GoTo => "Go to folder".into(),
            Dialog::Info { path, .. } => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
        }
    }

    #[cfg(test)]
    pub fn kind(&self) -> &'static str {
        match self {
            Dialog::Mask { .. } => "mask",
            Dialog::NewFolder { .. } => "new-folder",
            Dialog::NewFile { .. } => "new-file",
            Dialog::Rename { .. } => "rename",
            Dialog::Transfer { .. } => "transfer",
            Dialog::Delete { .. } => "delete",
            Dialog::GoTo => "goto",
            Dialog::Info { .. } => "info",
        }
    }

    fn subtitle(&self) -> String {
        match self {
            Dialog::Mask { .. } => "Use * and ? as wildcards, for example *.png".into(),
            Dialog::NewFolder { .. } | Dialog::NewFile { .. } | Dialog::Rename { .. } => {
                String::new()
            }
            Dialog::Transfer {
                conflicts: Some((dir, c)),
                ..
            } => format!(
                "{} in {}. Replace them or keep the existing files?",
                names(&c.iter().map(PathBuf::from).collect::<Vec<_>>()),
                dir.display()
            ),
            Dialog::Transfer { sources, .. } => format!("{} to this folder:", names(sources)),
            Dialog::Delete {
                sources,
                permanent: false,
            } => format!(
                "{}. You can restore {} from the Trash.",
                names(sources),
                if sources.len() == 1 { "it" } else { "them" }
            ),
            Dialog::Delete {
                sources,
                permanent: true,
            } => format!("{}. This can't be undone.", names(sources)),
            Dialog::GoTo => "Type or paste a path. ~ stands for your home folder.".into(),
            Dialog::Info { .. } => String::new(),
        }
    }

    pub fn has_input(&self) -> bool {
        !matches!(
            self,
            Dialog::Delete { .. }
                | Dialog::Info { .. }
                | Dialog::Transfer {
                    conflicts: Some(_),
                    ..
                }
        )
    }

    fn buttons(&self) -> Vec<(&'static str, Choice, Tone)> {
        match self {
            Dialog::Mask { selecting } => vec![(
                if *selecting { "Select" } else { "Deselect" },
                Choice::Confirm,
                Tone::Primary,
            )],
            Dialog::NewFolder { .. } | Dialog::NewFile { .. } => {
                vec![("Create", Choice::Confirm, Tone::Primary)]
            }
            Dialog::Rename { .. } => vec![("Rename", Choice::Confirm, Tone::Primary)],
            Dialog::Transfer {
                conflicts: Some(_), ..
            } => vec![
                ("Keep existing", Choice::Skip, Tone::Plain),
                ("Replace", Choice::Overwrite, Tone::Primary),
            ],
            Dialog::Transfer { kind, .. } => vec![(
                if *kind == BatchKind::Copy {
                    "Copy"
                } else {
                    "Move"
                },
                Choice::Confirm,
                Tone::Primary,
            )],
            Dialog::Delete {
                permanent: false, ..
            } => vec![("Move to Trash", Choice::Confirm, Tone::Danger)],
            Dialog::Delete {
                permanent: true, ..
            } => vec![("Delete", Choice::Confirm, Tone::Danger)],
            Dialog::GoTo => vec![("Go", Choice::Confirm, Tone::Primary)],
            Dialog::Info { .. } => Vec::new(),
        }
    }
}

impl Workspace {
    pub(super) fn open_dialog(
        &mut self,
        dialog: Dialog,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let has_input = dialog.has_input();
        self.dialog = Some(dialog);
        self.dialog_error = None;
        self.dialog_input.update(cx, |s, cx| {
            s.set_value(value.to_string(), window, cx);
            s.select_all(window, cx);
            if has_input {
                s.focus(window, cx);
            }
        });
        if !has_input {
            window.focus(&self.dialog_focus, cx);
        }
        cx.notify();
    }

    pub(super) fn open_mask(
        &mut self,
        selecting: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_dialog(Dialog::Mask { selecting }, "*", window, cx);
    }

    pub(super) fn close_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        self.dialog_error = None;
        self.focus_pane(self.active, window, cx);
        cx.notify();
    }

    pub(super) fn confirm_prompt(
        &mut self,
        _: &ConfirmPrompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let choice = match &self.dialog {
            Some(Dialog::Transfer {
                conflicts: Some(_), ..
            }) => Choice::Overwrite,
            _ => Choice::Confirm,
        };
        self.choose(choice, window, cx);
    }

    fn cancel_prompt(&mut self, _: &CancelPrompt, window: &mut Window, cx: &mut Context<Self>) {
        self.close_prompt(window, cx);
    }

    fn fail(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.dialog_error = Some(text.into());
        cx.notify();
    }

    pub(super) fn choose(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let value = self.dialog_input.read(cx).value().trim().to_string();
        match dialog {
            Dialog::Mask { selecting } => {
                if !value.is_empty() {
                    self.active_pane()
                        .clone()
                        .update(cx, |p, cx| p.apply_mask(&value, selecting, cx));
                }
                self.close_prompt(window, cx);
            }
            Dialog::NewFolder { dir } => self.create_entry(dir, value, true, window, cx),
            Dialog::NewFile { dir } => self.create_entry(dir, value, false, window, cx),
            Dialog::Rename { dir, old } => match validate_name(&value) {
                Err(e) => {
                    self.dialog = Some(Dialog::Rename { dir, old });
                    self.fail(e, cx);
                }
                Ok(name) if name == old => self.close_prompt(window, cx),
                Ok(name) => {
                    let name = name.to_string();
                    let src = dir.join(&old);
                    let dst = dir.join(&name);
                    let result =
                        if dst.symlink_metadata().is_ok() && !old.eq_ignore_ascii_case(&name) {
                            Err(format!("\u{201c}{name}\u{201d} already exists."))
                        } else {
                            oryn_core::services::fs_rename::rename(
                                &src.to_string_lossy(),
                                &dst.to_string_lossy(),
                            )
                            .map_err(|e| e.to_string())
                        };
                    match result {
                        Ok(()) => {
                            self.close_prompt(window, cx);
                            self.refresh_all(Some(&name), cx);
                        }
                        Err(e) => {
                            self.dialog = Some(Dialog::Rename { dir, old });
                            self.fail(e, cx);
                        }
                    }
                }
            },
            Dialog::Transfer {
                kind,
                sources,
                conflicts,
            } => {
                let dest = match conflicts {
                    Some((dir, _)) => dir,
                    None => PathBuf::from(&value),
                };
                if !dest.is_dir() {
                    self.dialog = Some(Dialog::Transfer {
                        kind,
                        sources,
                        conflicts: None,
                    });
                    return self.fail("That folder doesn't exist.", cx);
                }
                if batch::same_place(&sources, &dest) {
                    self.dialog = Some(Dialog::Transfer {
                        kind,
                        sources,
                        conflicts: None,
                    });
                    return self.fail("Pick a different folder, or use Duplicate.", cx);
                }
                let policy = match choice {
                    Choice::Skip => OverwritePolicy::Skip,
                    Choice::Overwrite => OverwritePolicy::Overwrite,
                    Choice::Confirm => {
                        let clash = batch::conflicts(&sources, &dest);
                        if !clash.is_empty() {
                            self.dialog = Some(Dialog::Transfer {
                                kind,
                                sources,
                                conflicts: Some((dest, clash)),
                            });
                            window.focus(&self.dialog_focus, cx);
                            cx.notify();
                            return;
                        }
                        OverwritePolicy::Overwrite
                    }
                };
                self.close_prompt(window, cx);
                self.start_batch(kind, sources, Some(dest), policy, cx);
            }
            Dialog::GoTo => {
                let here = self.active_pane().read(cx).path.clone();
                let target = super::palette::expand(&value).unwrap_or_else(|| here.join(&value));
                if target.is_dir() || oryn_core::vfs::is_archive_file(&target) {
                    self.close_prompt(window, cx);
                    self.go_to(target, window, cx);
                } else if target.is_file() {
                    self.close_prompt(window, cx);
                    self.active_pane()
                        .clone()
                        .update(cx, |p, cx| p.reveal(&target, cx));
                } else {
                    self.dialog = Some(Dialog::GoTo);
                    self.fail("That folder doesn't exist.", cx);
                }
            }
            Dialog::Info { .. } => self.close_prompt(window, cx),
            Dialog::Delete { sources, permanent } => {
                self.close_prompt(window, cx);
                let kind = if permanent {
                    BatchKind::Delete
                } else {
                    BatchKind::Trash
                };
                self.start_batch(kind, sources, None, OverwritePolicy::Overwrite, cx);
            }
        }
    }

    fn create_entry(
        &mut self,
        dir: PathBuf,
        value: String,
        folder: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let back = |dir| {
            if folder {
                Dialog::NewFolder { dir }
            } else {
                Dialog::NewFile { dir }
            }
        };
        let name = match validate_name(&value) {
            Ok(n) => n.to_string(),
            Err(e) => {
                self.dialog = Some(back(dir));
                return self.fail(e, cx);
            }
        };
        let path = dir.join(&name);
        let result = if path.symlink_metadata().is_ok() {
            Err(format!("\u{201c}{name}\u{201d} already exists."))
        } else if folder {
            std::fs::create_dir(&path).map_err(|e| e.to_string())
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        match result {
            Ok(()) => {
                self.close_prompt(window, cx);
                self.refresh_all(Some(&name), cx);
            }
            Err(e) => {
                self.dialog = Some(back(dir));
                self.fail(e, cx);
            }
        }
    }

    pub(super) fn render_dialog(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.dialog.as_ref()?;
        let button = |id: usize, label: &'static str, tone: Tone| {
            div()
                .id(("dialog-btn", id))
                .debug_selector(|| format!("dialog-{label}"))
                .role(Role::Button)
                .aria_label(label)
                .flex()
                .items_center()
                .justify_center()
                .h(px(30.0))
                .px(px(16.0))
                .rounded(px(6.0))
                .text_size(px(13.0))
                .cursor_pointer()
                .when(tone != Tone::Plain, |d| {
                    let c = if tone == Tone::Danger {
                        hex(0xff453a)
                    } else {
                        p.accent
                    };
                    let hover = if tone == Tone::Danger {
                        hex(0xe0382e)
                    } else {
                        p.accent_hover
                    };
                    d.bg(c)
                        .text_color(hex(0xffffff))
                        .font_weight(FontWeight::SEMIBOLD)
                        .shadow(vec![BoxShadow {
                            color: with_alpha(c, 0.35),
                            offset: point(px(0.0), px(2.0)),
                            blur_radius: px(8.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .hover(move |s| s.bg(hover))
                })
                .when(tone == Tone::Plain, |d| {
                    d.bg(white_alpha(0.08))
                        .border_1()
                        .border_color(white_alpha(0.1))
                        .text_color(hex(0xd1d1d6))
                        .hover(|s| s.bg(white_alpha(0.14)).text_color(p.text_strong))
                })
                .child(label)
        };
        let title = dialog.title();
        let subtitle = dialog.subtitle();
        let buttons = dialog.buttons();
        let close_label = if buttons.is_empty() {
            "Close"
        } else {
            "Cancel"
        };
        Some(
            div()
                .id("dialog-overlay")
                .key_context(if dialog.has_input() {
                    "Dialog"
                } else {
                    "ConfirmDialog"
                })
                .track_focus(&self.dialog_focus)
                .on_action(cx.listener(Self::cancel_prompt))
                .on_action(cx.listener(Self::confirm_prompt))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(black_alpha(0.55))
                .on_click(cx.listener(|this, _, w, cx| this.close_prompt(w, cx)))
                .child(
                    div()
                        .id("dialog")
                        .role(Role::Dialog)
                        .aria_label(SharedString::from(title.clone()))
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .w(px(440.0))
                        .flex()
                        .flex_col()
                        .py(px(20.0))
                        .px(px(24.0))
                        .rounded(px(14.0))
                        .bg(hex(0x242428).opacity(0.97))
                        .border_1()
                        .border_color(white_alpha(0.12))
                        .shadow(vec![BoxShadow {
                            color: black_alpha(0.75),
                            offset: point(px(0.0), px(28.0)),
                            blur_radius: px(70.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .child(
                            div()
                                .mb(px(4.0))
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_strong)
                                .child(title),
                        )
                        .when(subtitle.is_empty(), |d| d.child(div().h(px(10.0))))
                        .when(!subtitle.is_empty(), |d| {
                            d.child(
                                div()
                                    .mb(px(14.0))
                                    .text_size(px(12.5))
                                    .line_height(px(17.5))
                                    .text_color(p.text_muted)
                                    .child(subtitle),
                            )
                        })
                        .when_some(
                            match dialog {
                                Dialog::Info { rows, .. } => Some(rows.clone()),
                                _ => None,
                            },
                            |d, rows| {
                                d.child(
                                    div()
                                        .id("info-rows")
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.0))
                                        .p(px(12.0))
                                        .rounded(px(8.0))
                                        .bg(white_alpha(0.04))
                                        .border_1()
                                        .border_color(white_alpha(0.08))
                                        .text_size(px(12.5))
                                        .children(rows.into_iter().map(|(k, v)| {
                                            div()
                                                .flex()
                                                .gap(px(12.0))
                                                .child(
                                                    div()
                                                        .w(px(90.0))
                                                        .flex_none()
                                                        .text_color(p.text_muted)
                                                        .child(k),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .text_color(p.text_strong)
                                                        .child(v),
                                                )
                                        })),
                                )
                            },
                        )
                        .when(dialog.has_input(), |d| {
                            d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .h(px(34.0))
                                    .px(px(12.0))
                                    .rounded(px(8.0))
                                    .bg(white_alpha(0.1))
                                    .border_1()
                                    .border_color(p.accent)
                                    .shadow(vec![BoxShadow {
                                        color: with_alpha(p.accent, 0.28),
                                        offset: point(px(0.0), px(0.0)),
                                        blur_radius: px(0.0),
                                        spread_radius: px(3.0),
                                        inset: false,
                                    }])
                                    .child(
                                        Input::new(&self.dialog_input)
                                            .appearance(false)
                                            .text_size(px(13.0))
                                            .text_color(p.text_strong),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .h(px(18.0))
                                .mt(px(4.0))
                                .text_size(px(12.0))
                                .text_color(hex(0xff6961))
                                .children(self.dialog_error.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(10.0))
                                .child(button(0, close_label, Tone::Plain).on_click(
                                    cx.listener(|this, _, w, cx| this.close_prompt(w, cx)),
                                ))
                                .children(buttons.into_iter().enumerate().map(
                                    |(i, (label, choice, tone))| {
                                        button(i + 1, label, tone).on_click(cx.listener(
                                            move |this, _, w, cx| this.choose(choice, w, cx),
                                        ))
                                    },
                                )),
                        ),
                )
                .into_any_element(),
        )
    }
}
