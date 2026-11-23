mod actions;
mod icons;
mod pane;
mod terminal;
mod theme;
mod workspace;

use std::path::PathBuf;

use gpui_kit::*;

use crate::theme::OrynTheme;
use crate::workspace::Workspace;

fn start_dir(arg: Option<String>) -> PathBuf {
    arg.map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let left = start_dir(args.next());
    let right = start_dir(args.next());
    let tray = std::env::var("ORYN_TRAY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| oryn_core::settings::Settings::load().tray);
    let bench = std::env::var_os("ORYN_BENCH").is_some();
    let terminal = std::env::var_os("ORYN_OPEN_TERMINAL").is_some();

    gpui_kit::application()
        .with_assets(icons::AppAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.set_global(OrynTheme { tray });
            actions::bind_keys(cx);
            let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Oryn".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.0), px(18.0))),
                }),
                window_decorations: Some(WindowDecorations::Client),
                window_min_size: Some(size(px(900.0), px(560.0))),
                ..Default::default()
            };
            let (handle, ws) = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Workspace::new(left.clone(), right.clone(), window, cx))
            })
            .expect("open window");
            let _ = handle.update(cx, |_, window, cx| {
                if bench {
                    let pane = ws.read(cx).left();
                    pane.update(cx, |p, cx| p.start_bench(cx));
                }
                if terminal {
                    window.dispatch_action(Box::new(actions::ToggleTerminal), cx);
                }
            });
            cx.activate(true);
        });
}
