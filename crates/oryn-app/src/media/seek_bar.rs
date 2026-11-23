use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::*;

use crate::theme::{Palette, white_alpha};

type OnSeek = Rc<dyn Fn(f32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SeekBar {
    id: &'static str,
    fraction: f32,
    palette: Palette,
    on_seek: OnSeek,
}

impl SeekBar {
    pub fn new(
        id: &'static str,
        fraction: f32,
        palette: Palette,
        on_seek: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            fraction: fraction.clamp(0.0, 1.0),
            palette,
            on_seek: Rc::new(on_seek),
        }
    }
}

fn fraction_at(bounds: Bounds<Pixels>, x: Pixels) -> f32 {
    let width = bounds.size.width.as_f32();
    if width <= 0.0 {
        return 0.0;
    }
    ((x - bounds.origin.x).as_f32() / width).clamp(0.0, 1.0)
}

impl RenderOnce for SeekBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let p = self.palette;
        let bounds = Rc::new(Cell::new(Bounds::default()));
        let down = {
            let bounds = bounds.clone();
            let on_seek = self.on_seek.clone();
            move |ev: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                cx.stop_propagation();
                on_seek(fraction_at(bounds.get(), ev.position.x), window, cx);
            }
        };
        let drag = {
            let bounds = bounds.clone();
            let on_seek = self.on_seek.clone();
            move |ev: &MouseMoveEvent, window: &mut Window, cx: &mut App| {
                if ev.pressed_button == Some(MouseButton::Left) {
                    on_seek(fraction_at(bounds.get(), ev.position.x), window, cx);
                }
            }
        };
        let id = self.id;
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .relative()
            .flex_1()
            .h(px(18.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, down)
            .on_mouse_move(drag)
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .child(
                div()
                    .w_full()
                    .h(px(4.0))
                    .rounded(px(2.0))
                    .bg(white_alpha(0.14))
                    .child(
                        div()
                            .h_full()
                            .w(relative(self.fraction))
                            .rounded(px(2.0))
                            .bg(p.accent),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(self.fraction))
                    .ml(px(-6.0))
                    .size(px(12.0))
                    .rounded_full()
                    .bg(p.text_strong)
                    .border_2()
                    .border_color(p.accent),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, point, px, size};

    use super::fraction_at;

    #[test]
    fn maps_clicks_to_fractions() {
        let b = Bounds {
            origin: point(px(100.0), px(0.0)),
            size: size(px(200.0), px(18.0)),
        };
        assert_eq!(fraction_at(b, px(150.0)), 0.25);
        assert_eq!(fraction_at(b, px(20.0)), 0.0);
        assert_eq!(fraction_at(b, px(999.0)), 1.0);
        assert_eq!(fraction_at(Bounds::default(), px(5.0)), 0.0);
    }
}
