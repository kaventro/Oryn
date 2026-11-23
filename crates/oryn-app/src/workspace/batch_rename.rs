use std::path::PathBuf;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::ui::rename::{CaseTransform, Day, MultiRename as Options, NumberPosition, apply_plan, plan};

use super::Workspace;
use crate::actions::*;
use crate::theme::{Palette, black_alpha, hex, white_alpha, with_alpha};

pub struct BatchRename {
    dir: PathBuf,
    names: Vec<String>,
    others: Vec<String>,
    find: Entity<InputState>,
    replace: Entity<InputState>,
    prefix: Entity<InputState>,
    suffix: Entity<InputState>,
    regex: bool,
    case: CaseTransform,
    number: NumberPosition,
    start: u64,
    digits: usize,
    error: Option<SharedString>,
    focus: FocusHandle,
}

impl BatchRename {
    fn options(&self, cx: &App) -> Options {
        let v = |e: &Entity<InputState>| e.read(cx).value().to_string();
        Options {
            find: v(&self.find),
            replace: v(&self.replace),
            regex: self.regex,
            prefix: v(&self.prefix),
            suffix: v(&self.suffix),
            case: self.case,
            number: self.number,
            start: self.start,
            digits: self.digits,
        }
    }
}

impl Workspace {
    pub(super) fn open_batch_rename(
        &mut self,
        _: &MultiRename,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let targets = self.active_pane().read(cx).targets();
        let Some(dir) = targets.first().and_then(|p| p.parent()).map(PathBuf::from) else {
            self.active_pane().clone().update(cx, |p, cx| {
                p.report("Select the files to rename first.".into(), cx)
            });
            return;
        };
        let names: Vec<String> = targets
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        let others = oryn_core::list_dir(&dir)
            .map(|items| items.into_iter().map(|i| i.name).collect())
            .unwrap_or_default();
        let mut field = |placeholder: &'static str, cx: &mut Context<Self>| {
            let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            cx.subscribe(&state, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change)
                    && let Some(b) = &mut this.batch
                {
                    b.error = None;
                    cx.notify();
                }
            })
            .detach();
            state
        };
        let find = field("Text or pattern to find", cx);
        let replace = field("Replace with, for example [N]-[C]", cx);
        let prefix = field("Add before the name", cx);
        let suffix = field("Add after the name", cx);
        let first = find.clone();
        cx.defer_in(window, move |_, window, cx| {
            first.update(cx, |s, cx| s.focus(window, cx));
        });
        self.batch = Some(BatchRename {
            dir,
            names,
            others,
            find,
            replace,
            prefix,
            suffix,
            regex: false,
            case: CaseTransform::None,
            number: NumberPosition::None,
            start: 1,
            digits: 2,
            error: None,
            focus: cx.focus_handle(),
        });
        cx.notify();
    }

    fn close_batch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.batch = None;
        self.focus_pane(self.active, window, cx);
    }

    fn apply_batch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(b) = &self.batch else {
            return;
        };
        let planned = plan(&b.names, &b.options(cx), Day::today(), &b.others);
        match apply_plan(&b.dir, &planned) {
            Ok(n) => {
                let focus = planned.first().map(|p| p.new.clone());
                self.close_batch(window, cx);
                self.active_pane().clone().update(cx, |p, _| p.clear_marks());
                self.refresh_all(focus.as_deref(), cx);
                self.active_pane()
                    .clone()
                    .update(cx, |p, cx| p.report(format!("Renamed {n} item(s)."), cx));
            }
            Err(e) => {
                if let Some(b) = &mut self.batch {
                    b.error = Some(e.into());
                }
                cx.notify();
            }
        }
    }

    pub(super) fn render_batch_rename(
        &self,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let b = self.batch.as_ref()?;
        let planned = plan(&b.names, &b.options(cx), Day::today(), &b.others);
        let problems = planned.iter().filter(|x| x.problem.is_some()).count();
        let changes = planned.iter().filter(|x| x.old != x.new).count();

        let label = |text: &'static str| {
            div()
                .w(px(70.0))
                .flex_none()
                .text_size(px(12.0))
                .text_color(p.text_muted)
                .child(text)
        };
        let field = |state: &Entity<InputState>| {
            div()
                .flex_1()
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
        };
        let seg = |id: &'static str, text: &'static str, on: bool| {
            div()
                .id(id)
                .role(Role::RadioButton)
                .aria_label(text)
                .aria_selected(on)
                .px(px(10.0))
                .h(px(26.0))
                .flex()
                .items_center()
                .text_size(px(12.0))
                .cursor_pointer()
                .text_color(if on { p.on_accent } else { p.text_secondary })
                .when(on, |d| d.bg(p.accent))
                .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.08))))
                .child(text)
        };
        let group = || {
            div()
                .flex()
                .rounded(px(6.0))
                .overflow_hidden()
                .border_1()
                .border_color(white_alpha(0.12))
                .bg(white_alpha(0.04))
        };
        let row = |l: &'static str| div().flex().items_center().gap(px(10.0)).child(label(l));

        let cases = [
            ("case-none", "As is", CaseTransform::None),
            ("case-lower", "lower", CaseTransform::Lower),
            ("case-upper", "UPPER", CaseTransform::Upper),
            ("case-title", "Title", CaseTransform::Title),
            ("case-first", "First", CaseTransform::First),
        ];
        let numbers = [
            ("num-none", "None", NumberPosition::None),
            ("num-prefix", "Before", NumberPosition::Prefix),
            ("num-suffix", "After", NumberPosition::Suffix),
        ];
        let stepper = |id: &'static str, text: String, delta: fn(&mut BatchRename, i64)| {
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    div()
                        .id((id, 0usize))
                        .role(Role::Button)
                        .aria_label("Decrease")
                        .px(px(8.0))
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(white_alpha(0.1)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(b) = &mut this.batch {
                                delta(b, -1);
                            }
                            cx.notify();
                        }))
                        .child("\u{2212}"),
                )
                .child(
                    div()
                        .min_w(px(28.0))
                        .text_center()
                        .text_size(px(12.5))
                        .text_color(p.text_strong)
                        .child(text),
                )
                .child(
                    div()
                        .id((id, 1usize))
                        .role(Role::Button)
                        .aria_label("Increase")
                        .px(px(8.0))
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(white_alpha(0.1)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(b) = &mut this.batch {
                                delta(b, 1);
                            }
                            cx.notify();
                        }))
                        .child("+"),
                )
        };

        let preview = planned.iter().take(500).enumerate().map(|(i, x)| {
            let changed = x.old != x.new;
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(24.0))
                .px(px(10.0))
                .when(i % 2 == 1, |d| d.bg(white_alpha(0.025)))
                .text_size(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(p.text_secondary)
                        .child(x.old.clone()),
                )
                .child(div().text_color(p.text_muted).child("\u{2192}"))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(if x.problem.is_some() {
                            hex(0xff6961)
                        } else if changed {
                            p.text_strong
                        } else {
                            p.text_muted
                        })
                        .child(match x.problem {
                            Some(problem) => format!("{} ({problem})", x.new),
                            None => x.new.clone(),
                        }),
                )
        });

        let can_apply = problems == 0 && changes > 0;
        let status = b.error.clone().unwrap_or_else(|| {
            if problems > 0 {
                format!("{problems} name(s) need attention.").into()
            } else {
                format!("{changes} of {} will change.", planned.len()).into()
            }
        });

        Some(
            div()
                .id("batch-overlay")
                .key_context("Dialog")
                .track_focus(&b.focus)
                .on_action(cx.listener(|this, _: &CancelPrompt, w, cx| this.close_batch(w, cx)))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(black_alpha(0.55))
                .on_click(cx.listener(|this, _, w, cx| this.close_batch(w, cx)))
                .child(
                    div()
                        .id("batch-dialog")
                        .role(Role::Dialog)
                        .aria_label("Batch rename")
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .w(px(640.0))
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
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
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_strong)
                                .child(format!("Batch rename {} item(s)", b.names.len())),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.text_muted)
                                .child("Tokens: [N] name, [E] extension, [C] counter, [YMD], [Y], [M], [D]"),
                        )
                        .child(row("Find").child(field(&b.find)).child(
                            group().child(
                                seg("regex", "Regex", b.regex).on_click(cx.listener(
                                    |this, _, _, cx| {
                                        if let Some(b) = &mut this.batch {
                                            b.regex = !b.regex;
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                        ))
                        .child(row("Replace").child(field(&b.replace)))
                        .child(
                            row("Prefix")
                                .child(field(&b.prefix))
                                .child(label("Suffix"))
                                .child(field(&b.suffix)),
                        )
                        .child(row("Case").child(group().children(cases.into_iter().map(
                            |(id, text, case)| {
                                seg(id, text, b.case == case).on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if let Some(b) = &mut this.batch {
                                            b.case = case;
                                        }
                                        cx.notify();
                                    },
                                ))
                            },
                        ))))
                        .child(
                            row("Number")
                                .child(group().children(numbers.into_iter().map(
                                    |(id, text, pos)| {
                                        seg(id, text, b.number == pos).on_click(cx.listener(
                                            move |this, _, _, cx| {
                                                if let Some(b) = &mut this.batch {
                                                    b.number = pos;
                                                }
                                                cx.notify();
                                            },
                                        ))
                                    },
                                )))
                                .child(div().w(px(10.0)))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.text_muted)
                                        .child("Start"),
                                )
                                .child(stepper("start", b.start.to_string(), |b, d| {
                                    b.start = (b.start as i64 + d).max(0) as u64
                                }))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.text_muted)
                                        .child("Digits"),
                                )
                                .child(stepper("digits", b.digits.to_string(), |b, d| {
                                    b.digits = (b.digits as i64 + d).clamp(1, 8) as usize
                                })),
                        )
                        .child(
                            div()
                                .id("batch-preview")
                                .h(px(220.0))
                                .overflow_y_scroll()
                                .rounded(px(8.0))
                                .border_1()
                                .border_color(white_alpha(0.08))
                                .bg(black_alpha(0.15))
                                .py(px(4.0))
                                .children(preview),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(12.0))
                                        .text_color(if problems > 0 || b.error.is_some() {
                                            hex(0xff6961)
                                        } else {
                                            p.text_muted
                                        })
                                        .child(status),
                                )
                                .child(
                                    div()
                                        .id("batch-cancel")
                                        .role(Role::Button)
                                        .aria_label("Cancel")
                                        .px(px(16.0))
                                        .h(px(30.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(6.0))
                                        .text_size(px(13.0))
                                        .bg(white_alpha(0.08))
                                        .border_1()
                                        .border_color(white_alpha(0.1))
                                        .text_color(hex(0xd1d1d6))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(white_alpha(0.14)))
                                        .on_click(
                                            cx.listener(|this, _, w, cx| this.close_batch(w, cx)),
                                        )
                                        .child("Cancel"),
                                )
                                .child(
                                    div()
                                        .id("batch-apply")
                                        .role(Role::Button)
                                        .aria_label("Rename")
                                        .px(px(16.0))
                                        .h(px(30.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(6.0))
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .bg(p.accent)
                                        .text_color(p.on_accent)
                                        .shadow(vec![BoxShadow {
                                            color: with_alpha(p.accent, 0.35),
                                            offset: point(px(0.0), px(2.0)),
                                            blur_radius: px(8.0),
                                            spread_radius: px(0.0),
                                            inset: false,
                                        }])
                                        .when(!can_apply, |d| d.opacity(0.45))
                                        .when(can_apply, |d| {
                                            d.cursor_pointer()
                                                .hover(|s| s.bg(p.accent_hover))
                                                .on_click(cx.listener(|this, _, w, cx| {
                                                    this.apply_batch(w, cx)
                                                }))
                                        })
                                        .child("Rename"),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
