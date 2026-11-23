use std::path::Path;

use std::path::PathBuf;
use std::sync::OnceLock;

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Action, AnyWindowHandle, AppContext, Bounds, Entity, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Point, TestAppContext, VisualTestContext, WindowBounds, WindowOptions, px, size,
};

use crate::pane::ViewMode;
use crate::tools::ToolKind;
use crate::workspace::Workspace;

fn data_dir() -> &'static Path {
    static DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIR.get_or_init(|| tempfile::tempdir().unwrap()).path()
}

fn open(
    cx: &mut TestAppContext,
    left: &Path,
    right: &Path,
) -> (AnyWindowHandle, Entity<Workspace>) {
    let data = data_dir();
    unsafe {
        std::env::set_var("ORYN_NO_UPDATE_CHECK", "1");
        std::env::set_var("ORYN_NO_WATCH", "1");
        std::env::set_var("ORYN_DATA_DIR", data);
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
    send(cx, handle.0, crate::actions::ViewList);
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

fn frame(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.run_until_parked();
}

fn bounds_of(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    sel: &'static str,
) -> Bounds<gpui_kit::Pixels> {
    frame(cx, window);
    let mut v = VisualTestContext::from_window(window, cx);
    v.debug_bounds(sel)
        .unwrap_or_else(|| panic!("nothing on screen called {sel}"))
}

fn click(cx: &mut TestAppContext, window: AnyWindowHandle, sel: &'static str) {
    let at = bounds_of(cx, window, sel).center();
    let mut v = VisualTestContext::from_window(window, cx);
    v.simulate_click(at, Modifiers::none());
    v.run_until_parked();
}

fn double_click(cx: &mut TestAppContext, window: AnyWindowHandle, sel: &'static str) {
    let position = bounds_of(cx, window, sel).center();
    let mut v = VisualTestContext::from_window(window, cx);
    for click_count in 1..=2 {
        v.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::none(),
            button: MouseButton::Left,
            click_count,
            first_mouse: false,
        });
        v.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::none(),
            button: MouseButton::Left,
            click_count,
        });
    }
    v.run_until_parked();
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn pane_path(cx: &mut TestAppContext, ws: &Entity<Workspace>, side: usize) -> PathBuf {
    cx.update(|cx| ws.read(cx).pane_at(side).read(cx).path.clone())
}

fn cursor_to(cx: &mut TestAppContext, ws: &Entity<Workspace>, name: &str) {
    cx.update(|cx| {
        let pane = ws.read(cx).pane_at(0);
        pane.update(cx, |p, _| {
            p.cursor = p.rows.iter().position(|r| r.name.as_ref() == name).unwrap();
        });
    });
}

fn favorites(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Vec<PathBuf> {
    cx.update(|cx| ws.read(cx).favorite_paths())
}

#[gpui_kit::test]
fn favourites_are_added_from_the_plus_and_the_menu_then_removed(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    std::fs::create_dir(a.join("projects")).unwrap();
    let (window, ws) = open(cx, &a, &b);
    assert!(!favorites(cx, &ws).contains(&a));

    click(cx, window, "favorites-add");
    assert!(favorites(cx, &ws).contains(&a));
    let saved = std::fs::read_to_string(data_dir().join("favorites.json")).unwrap();
    assert!(saved.contains(a.to_str().unwrap()));

    cursor_to(cx, &ws, "projects");
    send(cx, window, crate::actions::OpenContextMenu);
    assert!(cx.update(|cx| ws.read(cx).menu_has("Add to Favourites")));
    click(cx, window, "menu-Add to Favourites");
    let projects = a.join("projects");
    assert!(favorites(cx, &ws).contains(&projects));

    let ix = favorites(cx, &ws)
        .iter()
        .position(|p| p == &projects)
        .unwrap();
    click(cx, window, leak(format!("fav-{ix}")));
    assert_eq!(pane_path(cx, &ws, 0), projects);

    click(cx, window, leak(format!("fav-remove-{ix}")));
    assert!(!favorites(cx, &ws).contains(&projects));
    let ix = favorites(cx, &ws).iter().position(|p| p == &a).unwrap();
    click(cx, window, leak(format!("fav-remove-{ix}")));
    assert!(!favorites(cx, &ws).contains(&a));
    let saved = std::fs::read_to_string(data_dir().join("favorites.json")).unwrap();
    assert!(!saved.contains(a.to_str().unwrap()));
}

#[gpui_kit::test]
fn files_open_with_enter_and_double_click(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    std::fs::write(a.join("other.md"), "# hi").unwrap();
    let (window, ws) = open(cx, &a, &b);
    cursor_to(cx, &ws, "notes.txt");
    send(cx, window, crate::actions::OpenEntry);
    assert!(crate::opener::OPENED.lock().contains(&a.join("notes.txt")));
    double_click(cx, window, "row-other.md");
    assert!(crate::opener::OPENED.lock().contains(&a.join("other.md")));
    assert_eq!(pane_path(cx, &ws, 0), a);
}

fn make_zip(path: &Path) {
    use std::io::Write;
    let file = std::fs::File::create(path).unwrap();
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file("docs/guide.txt", opts).unwrap();
    w.write_all(b"inside").unwrap();
    w.start_file("top.txt", opts).unwrap();
    w.write_all(b"top").unwrap();
    w.finish().unwrap();
}

#[gpui_kit::test]
fn zip_archives_open_like_folders(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let zip = a.join("bundle.zip");
    make_zip(&zip);
    let (window, ws) = open(cx, &a, &b);
    double_click(cx, window, "row-bundle.zip");
    assert_eq!(pane_path(cx, &ws, 0), zip);
    let names: Vec<String> = cx.update(|cx| {
        ws.read(cx)
            .pane_at(0)
            .read(cx)
            .rows
            .iter()
            .map(|r| r.name.to_string())
            .collect()
    });
    assert!(names.contains(&"docs".to_string()));
    assert!(names.contains(&"top.txt".to_string()));

    cursor_to(cx, &ws, "top.txt");
    send(cx, window, crate::actions::OpenEntry);
    let opened = crate::opener::OPENED.lock().clone();
    let copy = opened
        .iter()
        .rev()
        .find(|p| p.file_name().is_some_and(|n| n == "top.txt"))
        .expect("the file inside the zip was opened");
    assert_eq!(std::fs::read_to_string(copy).unwrap(), "top");

    send(cx, window, crate::actions::CopyToOther);
    send(cx, window, crate::actions::ConfirmPrompt);
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(b.join("top.txt")).unwrap(), "top");

    send(cx, window, crate::actions::DeleteItems);
    assert!(zip.is_file());
    assert!(!overlays(cx, &ws).contains(&"dialog"));

    double_click(cx, window, "row-docs");
    assert_eq!(pane_path(cx, &ws, 0), zip.join("docs"));
    send(cx, window, crate::actions::GoParent);
    send(cx, window, crate::actions::GoParent);
    assert_eq!(pane_path(cx, &ws, 0), a);
}

#[gpui_kit::test]
fn sidebar_and_toolbar_controls_respond(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    std::fs::create_dir(a.join("deep")).unwrap();
    let (window, ws) = open(cx, &a, &b);

    let before = cx.update(|cx| ws.read(cx).collapsed());
    click(cx, window, "section-tags");
    let after = cx.update(|cx| ws.read(cx).collapsed());
    assert_ne!(
        before.contains(&"tags".to_string()),
        after.contains(&"tags".to_string())
    );
    click(cx, window, "section-tags");

    click(cx, window, "tool-more");
    assert_eq!(overlays(cx, &ws), ["menu"]);
    click(cx, window, "menu-Grid View");
    assert!(overlays(cx, &ws).is_empty());
    assert_eq!(cx.update(|cx| ws.read(cx).view_mode()), ViewMode::Grid);
    send(cx, window, crate::actions::ViewList);

    click(cx, window, "path-edit");
    assert_eq!(cx.update(|cx| ws.read(cx).dialog_kind()), Some("goto"));
    let deep = a.join("deep");
    cx.update_window(window, |_, w, cx| {
        ws.update(cx, |ws, cx| {
            ws.set_dialog_value(deep.to_str().unwrap(), w, cx)
        })
    })
    .unwrap();
    click(cx, window, "dialog-Go");
    assert_eq!(pane_path(cx, &ws, 0), deep);

    click(cx, window, "remote-add");
    assert_eq!(
        cx.update(|cx| ws.read(cx).tool_kind()),
        Some(ToolKind::Sftp)
    );
    send(cx, window, crate::actions::CancelPrompt);
}

#[gpui_kit::test]
fn the_filter_clears_from_its_button(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    std::fs::write(a.join("zeta.txt"), "z").unwrap();
    let (window, ws) = open(cx, &a, &b);
    send(cx, window, crate::actions::FocusSearch);
    let mut v = VisualTestContext::from_window(window, cx);
    v.simulate_input("zeta");
    v.run_until_parked();
    let count =
        |cx: &mut TestAppContext| cx.update(|cx| ws.read(cx).pane_at(0).read(cx).rows.len());
    assert_eq!(count(cx), 2);
    click(cx, window, "filter-clear");
    assert_eq!(count(cx), 3);
}

#[gpui_kit::test]
fn git_status_and_tags_show_in_the_list(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(&a)
            .output()
            .unwrap()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    std::fs::write(a.join("notes.txt"), "changed").unwrap();
    std::fs::write(a.join("fresh.txt"), "new").unwrap();
    let mut tags = oryn_core::tools::tags::Tags::load();
    tags.toggle(&[a.join("fresh.txt")], "Blue");
    tags.save();
    let (window, ws) = open(cx, &a, &b);
    frame(cx, window);
    let (marks, tagged) = cx.update(|cx| {
        let pane = ws.read(cx).pane_at(0);
        let pane = pane.read(cx);
        (pane.git.clone(), pane.tags.clone())
    });
    assert_eq!(marks.get("notes.txt"), Some(&'M'));
    assert_eq!(marks.get("fresh.txt"), Some(&'?'));
    assert_eq!(tagged.get("fresh.txt"), Some(&vec!["Blue".to_string()]));
}

#[gpui_kit::test]
fn properties_cut_paste_swap_and_hidden_files(cx: &mut TestAppContext) {
    let (_tmp, a, b) = dirs();
    std::fs::write(a.join(".secret"), "s").unwrap();
    let (window, ws) = open(cx, &a, &b);

    cursor_to(cx, &ws, "notes.txt");
    send(cx, window, crate::actions::ShowProperties);
    assert_eq!(cx.update(|cx| ws.read(cx).dialog_kind()), Some("info"));
    let rows = cx.update(|cx| ws.read(cx).dialog_rows());
    assert!(
        rows.iter()
            .any(|(k, v)| k == "Size" && v.contains("5 bytes"))
    );
    click(cx, window, "dialog-Close");
    assert!(overlays(cx, &ws).is_empty());

    cursor_to(cx, &ws, "notes.txt");
    send(cx, window, crate::actions::CutItems);
    send(cx, window, crate::actions::SwitchPane);
    send(cx, window, crate::actions::PasteItems);
    cx.run_until_parked();
    assert!(b.join("notes.txt").is_file());
    assert!(!a.join("notes.txt").exists());

    send(cx, window, crate::actions::SwapPanes);
    assert_eq!(pane_path(cx, &ws, 0), b);
    assert_eq!(pane_path(cx, &ws, 1), a);

    let has_secret = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            ws.read(cx)
                .pane_at(1)
                .read(cx)
                .rows
                .iter()
                .any(|r| r.name.as_ref() == ".secret")
        })
    };
    let shown = has_secret(cx);
    send(cx, window, crate::actions::ToggleHidden);
    assert_ne!(has_secret(cx), shown);
    send(cx, window, crate::actions::ToggleHidden);
    assert_eq!(has_secret(cx), shown);
}
