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
}
