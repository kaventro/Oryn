use gpui_kit::{App, KeyBinding, actions};

actions!(
    oryn,
    [
        MoveUp,
        MoveDown,
        PageUp,
        PageDown,
        MoveHome,
        MoveEnd,
        OpenEntry,
        GoParent,
        GoBack,
        GoForward,
        ToggleMark,
        MarkAll,
        SwitchPane,
        TogglePanes,
        ToggleSidebar,
        ToggleTerminal,
        FocusSearch,
        CycleTray,
        Quit,
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some("FilePane")),
        KeyBinding::new("down", MoveDown, Some("FilePane")),
        KeyBinding::new("pageup", PageUp, Some("FilePane")),
        KeyBinding::new("pagedown", PageDown, Some("FilePane")),
        KeyBinding::new("home", MoveHome, Some("FilePane")),
        KeyBinding::new("end", MoveEnd, Some("FilePane")),
        KeyBinding::new("enter", OpenEntry, Some("FilePane")),
        KeyBinding::new("backspace", GoParent, Some("FilePane")),
        KeyBinding::new("alt-up", GoParent, Some("FilePane")),
        KeyBinding::new("alt-left", GoBack, Some("FilePane")),
        KeyBinding::new("alt-right", GoForward, Some("FilePane")),
        KeyBinding::new("space", ToggleMark, Some("FilePane")),
        KeyBinding::new("insert", ToggleMark, Some("FilePane")),
        KeyBinding::new(sec("a"), MarkAll, Some("FilePane")),
        KeyBinding::new("tab", SwitchPane, Some("FilePane")),
        KeyBinding::new(sec("f"), FocusSearch, None),
        KeyBinding::new(sec("\\"), TogglePanes, None),
        KeyBinding::new(sec("b"), ToggleSidebar, None),
        KeyBinding::new("ctrl-`", ToggleTerminal, None),
        KeyBinding::new("f9", ToggleTerminal, None),
        KeyBinding::new(sec(","), CycleTray, None),
        KeyBinding::new(sec("q"), Quit, None),
    ]);
}

fn sec(rest: &str) -> &'static str {
    let prefix = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    Box::leak(format!("{prefix}-{rest}").into_boxed_str())
}
