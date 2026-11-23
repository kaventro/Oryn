use std::borrow::Cow;

use gpui_kit::*;
use oryn_core::icons::{icon_color, icon_key};

use crate::theme::{Palette, hex};

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .flex_none()
        .size(px(size))
        .text_color(color)
}

pub fn oryn_icon(name: &str, size: f32, color: Hsla) -> Svg {
    icon(&format!("oryn/{name}"), size, color)
}

pub fn file_icon(name: &str, is_dir: bool, on_accent: bool, p: &Palette) -> Svg {
    let key = icon_key(name, is_dir);
    let color = if on_accent {
        p.on_accent
    } else if key == "folder" {
        p.accent
    } else {
        icon_color(key).map(hex).unwrap_or(p.text_muted)
    };
    oryn_icon(&format!("file-{key}"), 16.0, color)
}

gpui_kit::assets::icon_assets!(
    LucideIcons,
    [
        Folder,
        House,
        Monitor,
        Download,
        HardDrive,
        FileText,
        Terminal,
        ChevronRight,
        X,
        FolderOpen, FolderInput, Copy, CopyPlus, Pencil, TextCursorInput, Clipboard, Trash, ArchiveX, FolderPlus, FilePlus, ListChecks, RefreshCw, Search, GitBranch, GitCompare, Hash, Tag, Server, Settings, Command, Eye, Info, FolderSync, Files, Replace, ChartPie, Clock, Image, FileCode, Keyboard, Palette, CornerDownLeft
    ]
);

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/oryn/*.svg"]
struct OrynIcons;

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = OrynIcons::get(path) {
            return Ok(Some(file.data));
        }
        if let Some(data) = LucideIcons.load(path)? {
            return Ok(Some(data));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v: Vec<SharedString> = OrynIcons::iter()
            .filter(|n| n.starts_with(path))
            .map(|n| SharedString::from(n.to_string()))
            .collect();
        v.extend(LucideIcons.list(path)?);
        v.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(v)
    }
}
