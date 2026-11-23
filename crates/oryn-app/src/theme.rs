use gpui_kit::{App, Global, Hsla, Rgba, SharedString};

#[derive(Clone, Copy)]
pub struct Tray {
    pub name: &'static str,
    pub accent: u32,
    pub accent_hover: u32,
}

pub const TRAYS: [Tray; 6] = [
    Tray {
        name: "Graphite Blue",
        accent: 0x0a84ff,
        accent_hover: 0x0071e3,
    },
    Tray {
        name: "Ocean",
        accent: 0x00b4d8,
        accent_hover: 0x0096c7,
    },
    Tray {
        name: "Ember",
        accent: 0xff9f0a,
        accent_hover: 0xf0883e,
    },
    Tray {
        name: "Emerald",
        accent: 0x34c759,
        accent_hover: 0x28a745,
    },
    Tray {
        name: "Purple",
        accent: 0xbf5af2,
        accent_hover: 0xa838e0,
    },
    Tray {
        name: "Mono",
        accent: 0xd1d1d6,
        accent_hover: 0xe5e5ea,
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

pub fn black_alpha(a: f32) -> Hsla {
    Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a,
    }
    .into()
}

pub fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Hsla,
    pub card: Hsla,
    pub divider: Hsla,
    pub card_border: Hsla,
    pub control: Hsla,
    pub control_border: Hsla,
    pub text: Hsla,
    pub text_strong: Hsla,
    pub text_secondary: Hsla,
    pub text_sidebar: Hsla,
    pub text_crumb: Hsla,
    pub text_muted: Hsla,
    pub text_dim: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub on_accent: Hsla,
    pub accent_soft: Hsla,
    pub terminal_bg: Hsla,
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
        card: hex(0x242428),
        divider: white_alpha(0.06),
        card_border: white_alpha(0.07),
        control: white_alpha(0.05),
        control_border: white_alpha(0.08),
        text: hex(0xf5f5f7),
        text_strong: hex(0xffffff),
        text_secondary: hex(0xc4c4c8),
        text_sidebar: hex(0xd1d1d6),
        text_crumb: hex(0xa8a8ac),
        text_muted: hex(0x8e8e93),
        text_dim: hex(0x727276),
        text_faint: hex(0x636366),
        accent,
        accent_hover: hex(tray.accent_hover),
        on_accent,
        accent_soft: with_alpha(accent, 0.22),
        terminal_bg: hex(0x111214),
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
