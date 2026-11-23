use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use regex::{Regex, RegexBuilder};

const MAX_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct ReplaceQuery {
    pub root: PathBuf,
    pub find: String,
    pub replace: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub glob: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMatch {
    pub path: PathBuf,
    pub count: usize,
    pub sample: Vec<(usize, String, String)>,
}

fn pattern(q: &ReplaceQuery) -> Result<Regex, String> {
    if q.find.is_empty() {
        return Err("Type the text to find.".into());
    }
    let p = if q.regex {
        q.find.clone()
    } else {
        regex::escape(&q.find)
    };
    RegexBuilder::new(&p)
        .case_insensitive(!q.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|e| e.to_string())
}

fn glob(q: &ReplaceQuery) -> Result<Option<globset::GlobMatcher>, String> {
    let g = q.glob.trim();
    if g.is_empty() || g == "*" {
        return Ok(None);
    }
    globset::GlobBuilder::new(g)
        .case_insensitive(true)
        .build()
        .map(|g| Some(g.compile_matcher()))
        .map_err(|e| e.to_string())
}

fn replacement(q: &ReplaceQuery) -> String {
    if q.regex {
        q.replace.clone()
    } else {
        q.replace.replace('$', "$$")
    }
}

fn files(q: &ReplaceQuery) -> Result<impl Iterator<Item = PathBuf>, String> {
    let g = glob(q)?;
    Ok(walkdir::WalkDir::new(&q.root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.metadata().map(|m| m.len() <= MAX_BYTES).unwrap_or(false))
        .filter(move |e| g.as_ref().is_none_or(|g| g.is_match(e.file_name())))
        .map(|e| e.into_path()))
}

fn read_text(p: &Path) -> Option<String> {
    let bytes = fs::read(p).ok()?;
    if bytes.iter().take(8192).any(|b| *b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

pub fn preview(q: &ReplaceQuery, cancel: &AtomicBool) -> Result<Vec<FileMatch>, String> {
    let re = pattern(q)?;
    let rep = replacement(q);
    let mut out = Vec::new();
    for path in files(q)? {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Some(text) = read_text(&path) else {
            continue;
        };
        let count = re.find_iter(&text).count();
        if count == 0 {
            continue;
        }
        let sample = text
            .lines()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .take(3)
            .map(|(i, l)| {
                let before: String = l.trim().chars().take(160).collect();
                let after: String = re
                    .replace_all(l.trim(), rep.as_str())
                    .chars()
                    .take(160)
                    .collect();
                (i + 1, before, after)
            })
            .collect();
        out.push(FileMatch {
            path,
            count,
            sample,
        });
    }
    Ok(out)
}

pub fn apply(
    q: &ReplaceQuery,
    targets: &[PathBuf],
) -> Result<(usize, Vec<(String, String)>), String> {
    let re = pattern(q)?;
    let rep = replacement(q);
    let mut changed = 0;
    let mut failed = Vec::new();
    for path in targets {
        let Some(text) = read_text(path) else {
            failed.push((path.display().to_string(), "not a text file".into()));
            continue;
        };
        let next = re.replace_all(&text, rep.as_str());
        if next == text {
            continue;
        }
        let tmp = path.with_file_name(format!(
            ".{}.oryn-replace",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        let res = fs::write(&tmp, next.as_bytes()).and_then(|_| {
            if let Ok(meta) = fs::metadata(path) {
                let _ = fs::set_permissions(&tmp, meta.permissions());
            }
            fs::rename(&tmp, path)
        });
        match res {
            Ok(()) => changed += 1,
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                failed.push((path.display().to_string(), e.to_string()));
            }
        }
    }
    Ok((changed, failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_and_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::create_dir_all(r.join("src/.hidden")).unwrap();
        fs::write(r.join("src/a.rs"), "let colour = 1;\nlet Colour = 2;\n").unwrap();
        fs::write(r.join("src/b.txt"), "colour colour\n").unwrap();
        fs::write(r.join("src/.hidden/c.rs"), "colour\n").unwrap();
        fs::write(r.join("bin.dat"), [0u8, b'c', b'o']).unwrap();
        let mut q = ReplaceQuery {
            root: r.to_path_buf(),
            find: "colour".into(),
            replace: "color$1".into(),
            ..Default::default()
        };
        let hits = preview(&q, &AtomicBool::new(false)).unwrap();
        assert_eq!(hits.len(), 2);
        let a = hits.iter().find(|h| h.path.ends_with("a.rs")).unwrap();
        assert_eq!(a.count, 2);
        assert_eq!(a.sample[0].2, "let color$1 = 1;");

        q.glob = "*.rs".into();
        q.case_sensitive = true;
        q.replace = "color".into();
        let hits = preview(&q, &AtomicBool::new(false)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].count, 1);
        let targets: Vec<_> = hits.into_iter().map(|h| h.path).collect();
        assert_eq!(apply(&q, &targets).unwrap().0, 1);
        assert_eq!(
            fs::read_to_string(r.join("src/a.rs")).unwrap(),
            "let color = 1;\nlet Colour = 2;\n"
        );

        q.regex = true;
        q.glob.clear();
        q.find = r"(\w+) (\w+)".into();
        q.replace = "$2 $1".into();
        let t = [r.join("src/b.txt")];
        apply(&q, &t).unwrap();
        assert_eq!(fs::read_to_string(&t[0]).unwrap(), "colour colour\n");
        q.find.clear();
        assert!(preview(&q, &AtomicBool::new(false)).is_err());
    }
}
