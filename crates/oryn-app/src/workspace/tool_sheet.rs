use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::Workspace;
use crate::actions::*;
use crate::icons::icon;
use crate::theme::{Palette, black_alpha, hex, white_alpha, with_alpha};
use crate::tools::*;

pub struct ToolHost {
    kind: ToolKind,
    view: AnyView,
    focus: FocusHandle,
    _sub: Subscription,
}

impl ToolHost {
    #[cfg(test)]
    pub fn kind(&self) -> ToolKind {
        self.kind
    }
}

impl Workspace {
    fn host<T: Render + Focusable + EventEmitter<ToolEvent>>(
        &mut self,
        kind: ToolKind,
        entity: Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sub = cx.subscribe_in(&entity, window, |this, _, ev: &ToolEvent, w, cx| {
            this.on_tool_event(ev, w, cx)
        });
        let focus = entity.focus_handle(cx);
        window.focus(&focus, cx);
        self.tool = Some(ToolHost {
            kind,
            view: entity.into(),
            focus,
            _sub: sub,
        });
        cx.notify();
    }

    pub(super) fn open_tool(
        &mut self,
        kind: ToolKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_tool_with(kind, None, window, cx);
    }

    pub(super) fn open_tool_with(
        &mut self,
        kind: ToolKind,
        tag: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.active_pane().read(cx);
        let ctx = ToolContext {
            dir: pane.op_dir(),
            other: self.other_dir(cx),
            targets: pane.targets(),
            tag,
        };
        match kind {
            ToolKind::Search => {
                let e = cx.new(|cx| search::SearchTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Git => {
                let e = cx.new(|cx| git::GitTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Replace => {
                let e = cx.new(|cx| replace::ReplaceTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Compare => {
                let e = cx.new(|cx| compare::CompareTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Duplicates => {
                let e = cx.new(|cx| duplicates::DuplicatesTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Checksum => {
                let e = cx.new(|cx| checksum::ChecksumTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Disk => {
                let e = cx.new(|cx| disk::DiskTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Tags => {
                let e = cx.new(|cx| tags::TagsTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
            ToolKind::Sftp => {
                let e = cx.new(|cx| sftp::SftpTool::new(&ctx, window, cx));
                self.host(kind, e, window, cx)
            }
        }
    }

    fn close_tool(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.tool.take().map(|t| t.kind);
        if was == Some(ToolKind::Sftp) {
            self.reload_profiles();
        }
        if was == Some(ToolKind::Tags) {
            self.refresh_all(None, cx);
        }
        self.focus_pane(self.active, window, cx);
    }

    fn on_tool_event(&mut self, ev: &ToolEvent, window: &mut Window, cx: &mut Context<Self>) {
        match ev {
            ToolEvent::Reveal(path) => {
                self.close_tool(window, cx);
                let path = path.clone();
                self.active_pane()
                    .clone()
                    .update(cx, |p, cx| p.reveal(&path, cx));
            }
            ToolEvent::Open(dir) => {
                self.close_tool(window, cx);
                self.go_to(dir.clone(), window, cx);
            }
            ToolEvent::Changed => self.refresh_all(None, cx),
        }
    }

    pub(super) fn render_tool(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let host = self.tool.as_ref()?;
        let current = host.kind;
        let rail = ToolKind::ALL.iter().enumerate().map(|(ix, &kind)| {
            let on = kind == current;
            div()
                .id(("tool", ix))
                .role(Role::Tab)
                .aria_label(kind.label())
                .aria_selected(on)
                .flex()
                .items_center()
                .gap(px(9.0))
                .h(px(32.0))
                .px(px(10.0))
                .rounded(px(7.0))
                .text_size(px(12.5))
                .cursor_pointer()
                .text_color(if on { p.text_strong } else { p.text_sidebar })
                .when(on, |d| d.bg(with_alpha(p.accent, 0.22)))
                .when(!on, |d| d.hover(|s| s.bg(white_alpha(0.05))))
                .on_click(cx.listener(move |this, _, w, cx| {
                    if this.tool.as_ref().map(|t| t.kind) != Some(kind) {
                        this.open_tool(kind, w, cx)
                    }
                }))
                .child(icon(
                    kind.icon(),
                    14.0,
                    if on { p.accent } else { p.text_muted },
                ))
                .child(kind.label())
        });
        Some(
            div()
                .id("tool-overlay")
                .key_context("Dialog")
                .on_action(cx.listener(|this, _: &CancelPrompt, w, cx| this.close_tool(w, cx)))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .p(px(36.0))
                .bg(black_alpha(0.55))
                .on_click(cx.listener(|this, _, w, cx| this.close_tool(w, cx)))
                .child(
                    div()
                        .id("tool-sheet")
                        .role(Role::Dialog)
                        .aria_label(current.label())
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .size_full()
                        .max_w(px(1180.0))
                        .max_h(px(780.0))
                        .flex()
                        .rounded(px(14.0))
                        .bg(hex(0x1f1f23).opacity(0.98))
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
                                .w(px(190.0))
                                .h_full()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .p(px(10.0))
                                .bg(p.bg)
                                .border_r_1()
                                .border_color(p.divider)
                                .child(
                                    div()
                                        .px(px(10.0))
                                        .pt(px(6.0))
                                        .pb(px(10.0))
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.text_muted)
                                        .child("TOOLS"),
                                )
                                .children(rail)
                                .child(div().flex_1())
                                .child(
                                    div()
                                        .id("tool-close")
                                        .px(px(10.0))
                                        .py(px(6.0))
                                        .text_size(px(11.5))
                                        .text_color(p.text_muted)
                                        .cursor_pointer()
                                        .hover(|s| s.text_color(p.text_strong))
                                        .on_click(
                                            cx.listener(|this, _, w, cx| this.close_tool(w, cx)),
                                        )
                                        .child("Esc to close"),
                                ),
                        )
                        .child(
                            div()
                                .id("tool-body")
                                .track_focus(&host.focus)
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .p(px(20.0))
                                .child(host.view.clone()),
                        ),
                )
                .into_any_element(),
        )
    }
}
