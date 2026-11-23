use std::path::Path;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Action, AnyWindowHandle, AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds,
    WindowOptions, px, size,
};

use crate::workspace::Workspace;

fn open(
    cx: &mut TestAppContext,
    left: &Path,
    right: &Path,
) -> (AnyWindowHandle, Entity<Workspace>) {
    unsafe {
        std::env::set_var("ORYN_NO_UPDATE_CHECK", "1");
        std::env::set_var("ORYN_NO_WATCH", "1");
    }
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::actions::bind_keys(cx);
    });
    let (left, right) = (left.to_path_buf(), right.to_path_buf());
    let handle = cx.update(|cx| {
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(1440.0), px(900.0)),
        };
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Workspace::new(left, right, window, cx)),
        )
        .expect("open window")
    });
    (handle.0, handle.1)
}

fn dirs() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("notes.txt"), "hello").unwrap();
    (tmp, a, b)
}

fn send(cx: &mut TestAppContext, window: AnyWindowHandle, action: impl Action) {
    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(action), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn overlays(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Vec<&'static str> {
    cx.update(|cx| ws.read(cx).open_overlays())
}

#[gpui_kit::test]
fn overlays_open_from_their_shortcuts(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let (window, ws) = open(cx, &a, &b);
    assert!(overlays(cx, &ws).is_empty());
    send(cx, window, crate::actions::OpenSettings);
    assert_eq!(overlays(cx, &ws), ["settings"]);
    let (_tmp, a, b) = dirs();
    let (window, ws) = open(cx, &a, &b);
    send(cx, window, crate::actions::OpenPalette);
    assert_eq!(overlays(cx, &ws), ["palette"]);
    let (_tmp, a, b) = dirs();
    let (window, ws) = open(cx, &a, &b);
    send(cx, window, crate::actions::ShowSearch);
    assert_eq!(overlays(cx, &ws), ["tool"]);
}

#[gpui_kit::test]
fn quick_view_opens_on_a_file(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let (window, ws) = open(cx, &a, &b);
    send(cx, window, crate::actions::MoveDown);
    send(cx, window, crate::actions::QuickLook);
    assert_eq!(overlays(cx, &ws), ["quick"]);
}

#[gpui_kit::test]
fn context_menu_opens_from_the_keyboard(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let (window, ws) = open(cx, &a, &b);
    send(cx, window, crate::actions::OpenContextMenu);
    assert_eq!(overlays(cx, &ws), ["menu"]);
}

#[gpui_kit::test]
fn compress_and_extract_from_the_workspace(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let (window, _ws) = open(cx, &a, &b);
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.dispatch_action(Box::new(crate::actions::MoveDown), cx);
        window.dispatch_action(Box::new(crate::actions::CompressZip), cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(a.join("notes.zip").is_file());
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.dispatch_action(Box::new(crate::actions::MoveDown), cx);
        window.dispatch_action(Box::new(crate::actions::ExtractHere), cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        std::fs::read_to_string(a.join("notes").join("notes.txt")).unwrap(),
        "hello"
    );
}
