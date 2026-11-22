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
        ToggleMark,
        MarkAll,
        SwitchPane,
        ToggleTerminal,
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
        KeyBinding::new("space", ToggleMark, Some("FilePane")),
        KeyBinding::new("insert", ToggleMark, Some("FilePane")),
        KeyBinding::new(sec("a"), MarkAll, Some("FilePane")),
        KeyBinding::new("tab", SwitchPane, Some("FilePane")),
        KeyBinding::new("ctrl-`", ToggleTerminal, None),
        KeyBinding::new(sec("shift-t"), CycleTray, None),
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
