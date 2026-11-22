use gpui_kit::{App, Global, Hsla, Rgba, SharedString};

#[derive(Clone, Copy)]
pub struct Tray {
    pub name: &'static str,
    pub accent: u32,
}

pub const TRAYS: [Tray; 6] = [
    Tray {
        name: "Graphite Blue",
        accent: 0x0a84ff,
    },
    Tray {
        name: "Ocean",
        accent: 0x00b4d8,
    },
    Tray {
        name: "Ember",
        accent: 0xff9f0a,
    },
    Tray {
        name: "Emerald",
        accent: 0x34c759,
    },
    Tray {
        name: "Purple",
        accent: 0xbf5af2,
    },
    Tray {
        name: "Mono",
        accent: 0xd1d1d6,
    },
];

pub struct OrynTheme {
    pub tray: usize,
}

impl Global for OrynTheme {}

pub fn hex(v: u32) -> Hsla {
    Rgba {
        r: ((v >> 16) & 0xff) as f32 / 255.0,
        g: ((v >> 8) & 0xff) as f32 / 255.0,
        b: (v & 0xff) as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

pub fn white_alpha(a: f32) -> Hsla {
    Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a,
    }
    .into()
}

pub fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

pub struct Palette {
    pub bg: Hsla,
    pub sidebar: Hsla,
    pub card: Hsla,
    pub pane: Hsla,
    pub divider: Hsla,
    pub card_border: Hsla,
    pub hover: Hsla,
    pub cursor_row: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub accent_soft: Hsla,
    pub red: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub purple: Hsla,
    pub pink: Hsla,
    pub cyan: Hsla,
    pub terminal_bg: Hsla,
    pub tray_name: SharedString,
}

pub fn palette(cx: &App) -> Palette {
    let tray = cx
        .try_global::<OrynTheme>()
        .map(|t| TRAYS[t.tray % TRAYS.len()])
        .unwrap_or(TRAYS[0]);
    let accent = hex(tray.accent);
    let on_accent = if accent.l > 0.75 {
        hex(0x1c1c1e)
    } else {
        hex(0xffffff)
    };
    Palette {
        bg: hex(0x141416),
        sidebar: hex(0x141416),
        card: hex(0x242428),
        pane: hex(0x1a1a1d),
        divider: white_alpha(0.06),
        card_border: white_alpha(0.07),
        hover: white_alpha(0.04),
        cursor_row: white_alpha(0.08),
        text: hex(0xf5f5f7),
        text_muted: hex(0x8e8e93),
        text_faint: hex(0x636366),
        accent,
        on_accent,
        accent_soft: with_alpha(accent, 0.16),
        red: hex(0xff453a),
        yellow: hex(0xffd60a),
        green: hex(0x34c759),
        purple: hex(0xbf5af2),
        pink: hex(0xff375f),
        cyan: hex(0x30b0c7),
        terminal_bg: hex(0x111214),
        tray_name: tray.name.into(),
    }
}

pub fn ui_font() -> SharedString {
    if cfg!(target_os = "macos") {
        ".SystemUIFont".into()
    } else if cfg!(target_os = "windows") {
        "Segoe UI".into()
    } else {
        "Inter".into()
    }
}

pub fn mono_font() -> SharedString {
    if cfg!(target_os = "macos") {
        "Menlo".into()
    } else if cfg!(target_os = "windows") {
        "Consolas".into()
    } else {
        "JetBrains Mono".into()
    }
}
