use gpui_kit::{App, KeyBinding, actions};

actions!(
    oryn,
    [
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        ViewList,
        ViewGrid,
        ViewColumns,
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
        SelectByMask,
        DeselectByMask,
        InvertSelection,
        ClearSelection,
        SelectByExtension,
        ConfirmPrompt,
        CancelPrompt,
        SwitchPane,
        TogglePanes,
        ToggleSidebar,
        ToggleTerminal,
        FocusSearch,
        CycleTray,
        NewTab,
        CloseTab,
        NextTab,
        PrevTab,
        Quit,
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some("FilePane")),
        KeyBinding::new("down", MoveDown, Some("FilePane")),
        KeyBinding::new("left", MoveLeft, Some("FilePane")),
        KeyBinding::new("right", MoveRight, Some("FilePane")),
        KeyBinding::new(sec("1"), ViewList, None),
        KeyBinding::new(sec("2"), ViewGrid, None),
        KeyBinding::new(sec("3"), ViewColumns, None),
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
        KeyBinding::new("alt-s", SelectByMask, Some("FilePane")),
        KeyBinding::new("alt-d", DeselectByMask, Some("FilePane")),
        KeyBinding::new("alt-i", InvertSelection, Some("FilePane")),
        KeyBinding::new("alt-a", ClearSelection, Some("FilePane")),
        KeyBinding::new("alt-e", SelectByExtension, Some("FilePane")),
        KeyBinding::new("escape", CancelPrompt, Some("MaskPrompt")),
        KeyBinding::new("tab", SwitchPane, Some("FilePane")),
        KeyBinding::new(sec("f"), FocusSearch, None),
        KeyBinding::new(sec("\\"), TogglePanes, None),
        KeyBinding::new(sec("b"), ToggleSidebar, None),
        KeyBinding::new("ctrl-`", ToggleTerminal, None),
        KeyBinding::new("f9", ToggleTerminal, None),
        KeyBinding::new(sec(","), CycleTray, None),
        KeyBinding::new(sec("t"), NewTab, None),
        KeyBinding::new(sec("w"), CloseTab, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PrevTab, None),
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
