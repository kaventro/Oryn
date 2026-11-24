use gpui_kit::*;

use super::Workspace;
use crate::actions::*;
use crate::quick_view::{QuickEvent, QuickView};
use crate::theme::black_alpha;

impl Workspace {
    pub(super) fn quick_look(
        &mut self,
        _: &QuickLook,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quick.is_some() {
            return self.close_quick(window, cx);
        }
        let Some(path) = self.active_pane().read(cx).cursor_file() else {
            self.active_pane().clone().update(cx, |p, cx| {
                p.report("Put the cursor on a file to preview it.".into(), cx)
            });
            return;
        };
        let Some(path) = crate::pane::materialize(&path) else {
            return;
        };
        let view = cx.new(|cx| QuickView::new(path, cx));
        let sub = cx.subscribe_in(
            &view,
            window,
            |this, view, ev: &QuickEvent, w, cx| match ev {
                QuickEvent::Close => this.close_quick(w, cx),
                QuickEvent::Step(d) => {
                    let d = *d;
                    let next = this
                        .active_pane()
                        .clone()
                        .update(cx, |p, cx| p.step_file(d, cx));
                    if let Some(path) = next.and_then(|p| crate::pane::materialize(&p)) {
                        view.update(cx, |v, cx| {
                            v.load(path, cx);
                            cx.notify();
                        });
                    }
                }
            },
        );
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        self.quick = Some((view, sub));
        cx.notify();
    }

    fn close_quick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.quick = None;
        self.focus_pane(self.active, window, cx);
    }

    pub(super) fn render_quick(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (view, _) = self.quick.as_ref()?;
        Some(
            div()
                .id("quick-overlay")
                .occlude()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .p(px(40.0))
                .bg(black_alpha(0.5))
                .on_click(cx.listener(|this, _, w, cx| this.close_quick(w, cx)))
                .child(
                    div()
                        .id("quick-frame")
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .size_full()
                        .max_w(px(1100.0))
                        .max_h(px(820.0))
                        .child(view.clone()),
                )
                .into_any_element(),
        )
    }
}
