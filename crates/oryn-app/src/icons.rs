use std::borrow::Cow;

use gpui_kit::*;
use oryn_core::format::FileKind;

use crate::theme::Palette;

pub fn icon(name: &'static str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .flex_none()
        .size(px(size))
        .text_color(color)
}

pub fn kind_icon(kind: FileKind, is_parent: bool, marked: bool, p: &Palette) -> Svg {
    if is_parent {
        return icon(
            "undo-2",
            15.0,
            if marked { p.on_accent } else { p.text_muted },
        );
    }
    let (name, color) = match kind {
        FileKind::Folder => ("folder", p.accent),
        FileKind::Image => ("file-image", p.purple),
        FileKind::Video => ("file-video-camera", p.pink),
        FileKind::Audio => ("file-music", p.pink),
        FileKind::Archive => ("file-archive", p.yellow),
        FileKind::Pdf => ("file-text", p.red),
        FileKind::Markdown => ("file-type", p.cyan),
        FileKind::Code => ("file-code", p.green),
        FileKind::Text => ("file-text", p.text_muted),
        FileKind::Other => ("file", p.text_muted),
    };
    icon(name, 16.0, if marked { p.on_accent } else { color })
}

gpui_kit::assets::icon_assets!(
    OrynIcons,
    [
        Folder,
        File,
        FileImage,
        FileVideoCamera,
        FileMusic,
        FileArchive,
        FileText,
        FileType,
        FileCode,
        Undo2,
        House,
        Monitor,
        Download,
        HardDrive,
        Palette,
        SquareTerminal,
        Terminal,
        ChevronRight,
    ]
);

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(data) = OrynIcons.load(path)? {
            return Ok(Some(data));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = OrynIcons.list(path)?;
        v.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(v)
    }
}
