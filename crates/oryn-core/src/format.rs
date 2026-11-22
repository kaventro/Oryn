use chrono::{DateTime, Local};

pub fn fmt_size(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return String::new();
    };
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{} kB", (bytes as f64 / KB as f64).round() as u64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

pub fn row_date_text(mtime: &str) -> String {
    match DateTime::parse_from_rfc3339(mtime) {
        Ok(d) => d
            .with_timezone(&Local)
            .format("%-d %b %Y at %H:%M")
            .to_string(),
        Err(_) => "\u{2014}".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Folder,
    Image,
    Video,
    Audio,
    Archive,
    Pdf,
    Markdown,
    Code,
    Text,
    Other,
}

pub fn file_kind(name: &str, is_dir: bool) -> FileKind {
    if is_dir {
        return FileKind::Folder;
    }
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "heic" | "tiff" => {
            FileKind::Image
        }
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => FileKind::Video,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => FileKind::Audio,
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "dmg" => {
            FileKind::Archive
        }
        "pdf" => FileKind::Pdf,
        "md" | "markdown" | "mdx" => FileKind::Markdown,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "h" | "cpp" | "hpp" | "java"
        | "kt" | "swift" | "rb" | "sh" | "toml" | "json" | "yaml" | "yml" | "css" | "html"
        | "lock" => FileKind::Code,
        "txt" | "log" | "csv" | "rtf" => FileKind::Text,
        _ => FileKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_match_the_web_ui() {
        assert_eq!(fmt_size(None), "");
        assert_eq!(fmt_size(Some(512)), "512 B");
        assert_eq!(fmt_size(Some(2048)), "2 kB");
        assert_eq!(fmt_size(Some(4_823_449)), "4.6 MB");
        assert_eq!(fmt_size(Some(3 * 1024 * 1024 * 1024)), "3.00 GB");
    }

    #[test]
    fn bad_dates_render_a_dash() {
        assert_eq!(row_date_text("not a date"), "\u{2014}");
    }

    #[test]
    fn kinds_follow_extensions() {
        assert_eq!(file_kind("Fonts", true), FileKind::Folder);
        assert_eq!(file_kind("Frame 12.PNG", false), FileKind::Image);
        assert_eq!(file_kind("notes.md", false), FileKind::Markdown);
        assert_eq!(file_kind("main.rs", false), FileKind::Code);
        assert_eq!(file_kind("README", false), FileKind::Other);
    }
}
