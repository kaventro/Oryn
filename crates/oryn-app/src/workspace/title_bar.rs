use std::rc::Rc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::Workspace;
use crate::chrome::WindowChrome;
use crate::icons::oryn_icon;
use crate::theme::{hex, white_alpha};

impl Workspace {
    #[cfg(test)]
    pub fn use_chrome(&mut self, chrome: Rc<dyn WindowChrome>, cx: &mut Context<Self>) {
        self.chrome = chrome;
        cx.notify();
    }

    pub(super) fn chrome(&self) -> Rc<dyn WindowChrome> {
        self.chrome.clone()
    }

    pub(super) fn drag_region(
        &self,
        region: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        region.window_control_area(WindowControlArea::Drag).when(
            self.chrome.app_drives_window(),
            |d| {
                d.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, e: &MouseDownEvent, window, _| {
                        this.drag_armed = e.click_count < 2;
                        if e.click_count == 2 {
                            this.chrome.zoom(window);
                        }
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseUpEvent, _, _| this.drag_armed = false),
                )
                .on_mouse_move(cx.listener(
                    |this, e: &MouseMoveEvent, window, _| {
                        let pressed = e.pressed_button == Some(MouseButton::Left);
                        if this.drag_armed && pressed {
                            this.chrome.start_move(window);
                        }
                        this.drag_armed &= !pressed;
                    },
                ))
            },
        )
    }

    pub(super) fn caption(
        &self,
        id: &'static str,
        area: WindowControlArea,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .occlude()
            .window_control_area(area)
            .flex()
            .items_center()
            .justify_center()
            .w(px(30.0))
            .h(px(28.0))
            .rounded(px(4.0))
            .hover(|s| s.bg(white_alpha(0.08)))
            .when(self.chrome.app_drives_window(), |d| {
                d.on_click(cx.listener(move |this, _, window, cx| {
                    this.chrome().press_caption(area, window, cx)
                }))
            })
            .child(oryn_icon(id, 12.0, hex(0xa1a1a6)))
    }
}
