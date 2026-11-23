use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use regex::{Regex, RegexBuilder};

use crate::search::matcher::build_name_matcher;

const MAX_CONTENT_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct FindQuery {
    pub root: PathBuf,
    pub name: String,
    pub content: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub hidden: bool,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub line: Option<(usize, String)>,
}

fn name_mode(name: &str, regex: bool) -> &'static str {
    if regex {
        "regex"
    } else if name.contains(['*', '?', '[']) {
        "glob"
    } else {
        "substring"
    }
}

fn content_regex(q: &FindQuery) -> Result<Option<Regex>, String> {
    if q.content.is_empty() {
        return Ok(None);
    }
    let pattern = if q.regex {
        q.content.clone()
    } else {
        regex::escape(&q.content)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!q.case_sensitive)
        .build()
        .map(Some)
        .map_err(|e| e.to_string())
}

fn looks_binary(path: &Path) -> bool {
    let mut buf = [0u8; 4096];
    match fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => buf[..n].contains(&0),
        Err(_) => true,
    }
}

fn first_match(path: &Path, re: &Regex) -> Option<(usize, String)> {
    let file = fs::File::open(path).ok()?;
    for (ix, line) in BufReader::new(file).lines().enumerate() {
        let line = line.ok()?;
        if re.is_match(&line) {
            let text: String = line.trim().chars().take(200).collect();
            return Some((ix + 1, text));
        }
    }
    None
}

pub fn find(
    q: &FindQuery,
    cancel: &AtomicBool,
    mut on_hit: impl FnMut(Hit),
) -> Result<usize, String> {
    let parts: Vec<String> = q
        .name
        .split(if q.regex { '\u{0}' } else { ' ' })
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let name_match = if parts.is_empty() {
        None
    } else {
        Some(build_name_matcher(
            &parts,
            name_mode(&q.name, q.regex),
            q.case_sensitive,
        )?)
    };
    let content = content_regex(q)?;
    if name_match.is_none() && content.is_none() {
        return Ok(0);
    }
    let limit = if q.limit == 0 { 5000 } else { q.limit };
    let mut found = 0;
    let walker = walkdir::WalkDir::new(&q.root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| q.hidden || !e.file_name().to_string_lossy().starts_with('.'));
    for entry in walker.filter_map(Result::ok) {
        if cancel.load(Ordering::Relaxed) || found >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy();
        let is_dir = entry.file_type().is_dir();
        if let Some(m) = &name_match {
            if !m(&name, is_dir) {
                continue;
            }
        }
        let meta = entry.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let line = match &content {
            None => None,
            Some(re) => {
                if is_dir || !entry.file_type().is_file() || size > MAX_CONTENT_BYTES {
                    continue;
                }
                if looks_binary(entry.path()) {
                    continue;
                }
                match first_match(entry.path(), re) {
                    Some(l) => Some(l),
                    None => continue,
                }
            }
        };
        found += 1;
        on_hit(Hit {
            path: entry.path().to_path_buf(),
            is_dir,
            size: if is_dir { 0 } else { size },
            line,
        });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(q: &FindQuery) -> Vec<String> {
        let mut v = Vec::new();
        find(q, &AtomicBool::new(false), |h| {
            v.push(
                h.path
                    .strip_prefix(&q.root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            )
        })
        .unwrap();
        v.sort();
        v
    }

    #[test]
    fn finds_by_name_glob_and_content() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src/.git")).unwrap();
        fs::write(
            root.join("src/main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        fs::write(root.join("notes.md"), "hello world\n").unwrap();
        fs::write(root.join("src/.git/HEAD"), "hello\n").unwrap();
        fs::write(root.join("blob.bin"), [0u8, 1, 2, b'h', b'e']).unwrap();

        let mut q = FindQuery {
            root: root.to_path_buf(),
            name: "*.rs".into(),
            ..Default::default()
        };
        assert_eq!(run(&q), ["src/lib.rs", "src/main.rs"]);

        q.name = "MAIN".into();
        assert_eq!(run(&q), ["src/main.rs"]);

        q.name.clear();
        q.content = "hello".into();
        assert_eq!(run(&q), ["notes.md", "src/lib.rs", "src/main.rs"]);

        q.hidden = true;
        assert_eq!(run(&q).len(), 4);

        q.hidden = false;
        q.case_sensitive = true;
        q.content = "Hello".into();
        let mut hits = Vec::new();
        find(&q, &AtomicBool::new(false), |h| hits.push(h)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, Some((2, "println!(\"Hello\");".into())));

        q.content = "(".into();
        assert!(find(&q, &AtomicBool::new(false), |_| {}).is_ok());
        q.regex = true;
        assert!(find(&q, &AtomicBool::new(false), |_| {}).is_err());
        q.content = "fn \\w+\\(\\)".into();
        q.case_sensitive = false;
        assert_eq!(run(&q), ["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn empty_query_finds_nothing_and_cancel_stops() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a"), "").unwrap();
        let q = FindQuery {
            root: tmp.path().to_path_buf(),
            ..Default::default()
        };
        assert_eq!(run(&q), Vec::<String>::new());
        let q = FindQuery {
            name: "a".into(),
            ..q
        };
        assert_eq!(find(&q, &AtomicBool::new(true), |_| {}).unwrap(), 0);
    }
}
