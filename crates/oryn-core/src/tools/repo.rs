use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    pub staged: char,
    pub unstaged: char,
}

impl Change {
    pub fn is_staged(&self) -> bool {
        !matches!(self.staged, ' ' | '?')
    }

    pub fn is_unstaged(&self) -> bool {
        self.unstaged != ' '
    }

    pub fn label(&self) -> &'static str {
        let c = if self.is_staged() {
            self.staged
        } else {
            self.unstaged
        };
        match c {
            'M' => "Modified",
            'A' => "Added",
            'D' => "Deleted",
            'R' => "Renamed",
            'C' => "Copied",
            'U' => "Conflict",
            '?' => "Untracked",
            _ => "Changed",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<Change>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub when: String,
    pub subject: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Hunk,
    Context,
    Added,
    Removed,
    Meta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
}

fn command(root: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

pub fn run(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = command(root, args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Git is not installed.".to_string()
        } else {
            e.to_string()
        }
    })?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("git {} failed", args.first().unwrap_or(&""))
        } else {
            err
        })
    }
}

pub fn root(dir: &Path) -> Option<PathBuf> {
    run(dir, &["rev-parse", "--show-toplevel"])
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
}

pub fn parse_status(raw: &str) -> Status {
    let mut status = Status::default();
    let mut parts = raw.split('\0').peekable();
    while let Some(entry) = parts.next() {
        if let Some(head) = entry.strip_prefix("## ") {
            let (names, counts) = match head.find(" [") {
                Some(i) => (&head[..i], &head[i + 2..head.len().saturating_sub(1)]),
                None => (head, ""),
            };
            match names.split_once("...") {
                Some((b, u)) => {
                    status.branch = b.into();
                    status.upstream = Some(u.into());
                }
                None => {
                    status.branch = names
                        .strip_prefix("No commits yet on ")
                        .unwrap_or(names)
                        .into()
                }
            }
            for part in counts.split(", ") {
                if let Some(n) = part.strip_prefix("ahead ") {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("behind ") {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
            continue;
        }
        if entry.len() < 4 {
            continue;
        }
        let mut chars = entry.chars();
        let staged = chars.next().unwrap_or(' ');
        let unstaged = chars.next().unwrap_or(' ');
        if matches!(staged, 'R' | 'C') {
            parts.next();
        }
        status.changes.push(Change {
            path: entry[3..].to_string(),
            staged,
            unstaged,
        });
    }
    status
}

pub fn status(root: &Path) -> Result<Status, String> {
    run(root, &["status", "--porcelain=v1", "-b", "-z", "-uall"]).map(|s| parse_status(&s))
}

pub fn stage(root: &Path, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["add", "-A", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(root, &args).map(|_| ())
}

pub fn unstage(root: &Path, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["reset", "-q", "HEAD", "--"];
    args.extend(paths.iter().map(String::as_str));
    match run(root, &args) {
        Ok(_) => Ok(()),
        Err(_) => {
            let mut args = vec!["rm", "--cached", "-q", "-r", "--"];
            args.extend(paths.iter().map(String::as_str));
            run(root, &args).map(|_| ())
        }
    }
}

pub fn discard(root: &Path, change: &Change) -> Result<(), String> {
    if change.unstaged == '?' {
        let p = root.join(&change.path);
        return if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        }
        .map_err(|e| e.to_string());
    }
    run(root, &["checkout", "--", &change.path]).map(|_| ())
}

pub fn commit(root: &Path, message: &str) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Write a commit message first.".into());
    }
    run(root, &["commit", "-q", "-m", message])?;
    run(root, &["rev-parse", "--short", "HEAD"]).map(|s| s.trim().to_string())
}

pub fn branches(root: &Path) -> Result<Vec<String>, String> {
    Ok(run(root, &["branch", "--format=%(refname:short)"])?
        .lines()
        .map(str::to_string)
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn checkout(root: &Path, branch: &str, create: bool) -> Result<(), String> {
    if create {
        run(root, &["checkout", "-q", "-b", branch])
    } else {
        run(root, &["checkout", "-q", branch])
    }
    .map(|_| ())
}

pub fn pull(root: &Path) -> Result<String, String> {
    run(root, &["pull", "--ff-only"]).map(|s| s.trim().to_string())
}

pub fn push(root: &Path) -> Result<String, String> {
    run(root, &["push"]).map(|s| s.trim().to_string())
}

pub fn log(root: &Path, limit: usize) -> Result<Vec<Commit>, String> {
    let n = format!("-{limit}");
    let raw = run(
        root,
        &[
            "log",
            &n,
            "--date=relative",
            "--format=%h%x1f%an%x1f%ad%x1f%s%x1e",
        ],
    )?;
    Ok(raw
        .split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim().split('\x1f').collect();
            (f.len() == 4).then(|| Commit {
                hash: f[0].into(),
                author: f[1].into(),
                when: f[2].into(),
                subject: f[3].into(),
            })
        })
        .collect())
}

pub fn parse_diff(raw: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let (mut old, mut new) = (0u32, 0u32);
    let mut in_hunk = false;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("@@") {
            let mut nums = rest.split_whitespace();
            let parse = |s: Option<&str>, sign: char| {
                s.and_then(|s| s.strip_prefix(sign))
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            };
            old = parse(nums.next(), '-');
            new = parse(nums.next(), '+');
            in_hunk = true;
            out.push(DiffLine {
                kind: LineKind::Hunk,
                old: None,
                new: None,
                text: line.to_string(),
            });
            continue;
        }
        if !in_hunk {
            out.push(DiffLine {
                kind: LineKind::Meta,
                old: None,
                new: None,
                text: line.to_string(),
            });
            continue;
        }
        let (kind, o, n) = match line.chars().next() {
            Some('+') => {
                new += 1;
                (LineKind::Added, None, Some(new - 1))
            }
            Some('-') => {
                old += 1;
                (LineKind::Removed, Some(old - 1), None)
            }
            Some('\\') => (LineKind::Meta, None, None),
            _ => {
                old += 1;
                new += 1;
                (LineKind::Context, Some(old - 1), Some(new - 1))
            }
        };
        out.push(DiffLine {
            kind,
            old: o,
            new: n,
            text: line.get(1..).unwrap_or("").to_string(),
        });
    }
    out
}

pub fn diff(root: &Path, change: &Change) -> Result<Vec<DiffLine>, String> {
    let raw = if change.unstaged == '?' {
        let full = root.join(&change.path);
        let text = std::fs::read(&full).map_err(|e| e.to_string())?;
        if text.contains(&0) {
            return Ok(vec![DiffLine {
                kind: LineKind::Meta,
                old: None,
                new: None,
                text: "Binary file".into(),
            }]);
        }
        let text = String::from_utf8_lossy(&text);
        let lines: Vec<&str> = text.lines().collect();
        let mut s = format!("@@ -0,0 +1,{} @@\n", lines.len());
        for l in lines {
            s.push('+');
            s.push_str(l);
            s.push('\n');
        }
        s
    } else if change.is_staged() && !change.is_unstaged() {
        run(
            root,
            &["diff", "--cached", "--no-color", "--", &change.path],
        )?
    } else {
        run(root, &["diff", "--no-color", "--", &change.path])?
    };
    Ok(parse_diff(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_porcelain_status() {
        let raw = "## main...origin/main [ahead 2, behind 1]\0M  src/a.rs\0 M b.txt\0R  new.rs\0old.rs\0?? notes/x.md\0";
        let s = parse_status(raw);
        assert_eq!(s.branch, "main");
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.changes.len(), 4);
        assert!(s.changes[0].is_staged() && !s.changes[0].is_unstaged());
        assert!(!s.changes[1].is_staged() && s.changes[1].is_unstaged());
        assert_eq!(s.changes[2].path, "new.rs");
        assert_eq!(s.changes[2].label(), "Renamed");
        assert_eq!(s.changes[3].label(), "Untracked");
        assert!(!s.changes[3].is_staged());
        let fresh = parse_status("## No commits yet on main\0");
        assert_eq!(fresh.branch, "main");
    }

    #[test]
    fn parses_unified_diff() {
        let raw = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -3,3 +3,3 @@ fn x\n a\n-b\n+c\n d\n";
        let d = parse_diff(raw);
        assert_eq!(d[0].kind, LineKind::Meta);
        assert_eq!(d[3].kind, LineKind::Hunk);
        assert_eq!((d[4].old, d[4].new), (Some(3), Some(3)));
        assert_eq!((d[5].kind, d[5].old), (LineKind::Removed, Some(4)));
        assert_eq!((d[6].kind, d[6].new), (LineKind::Added, Some(4)));
        assert_eq!((d[7].old, d[7].new), (Some(5), Some(5)));
        assert_eq!(d[6].text, "c");
    }

    #[test]
    fn round_trips_against_real_git() {
        if run(Path::new("."), &["--version"]).is_err() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        run(r, &["init", "-q", "-b", "main"]).unwrap();
        run(r, &["config", "user.email", "t@example.com"]).unwrap();
        run(r, &["config", "user.name", "T"]).unwrap();
        run(r, &["config", "commit.gpgsign", "false"]).unwrap();
        fs::write(r.join("a.txt"), "one\n").unwrap();
        assert_eq!(
            fs::canonicalize(root(r).unwrap()).unwrap(),
            fs::canonicalize(r).unwrap()
        );
        let s = status(r).unwrap();
        assert_eq!(s.changes[0].label(), "Untracked");
        assert_eq!(diff(r, &s.changes[0]).unwrap().last().unwrap().text, "one");
        stage(r, &["a.txt".into()]).unwrap();
        assert!(status(r).unwrap().changes[0].is_staged());
        unstage(r, &["a.txt".into()]).unwrap();
        assert!(!status(r).unwrap().changes[0].is_staged());
        stage(r, &["a.txt".into()]).unwrap();
        assert!(commit(r, " ").is_err());
        commit(r, "first").unwrap();
        fs::write(r.join("a.txt"), "two\n").unwrap();
        let s = status(r).unwrap();
        let d = diff(r, &s.changes[0]).unwrap();
        assert!(d
            .iter()
            .any(|l| l.kind == LineKind::Added && l.text == "two"));
        discard(r, &s.changes[0]).unwrap();
        assert!(status(r).unwrap().changes.is_empty());
        checkout(r, "feature", true).unwrap();
        assert_eq!(branches(r).unwrap(), ["feature", "main"]);
        assert_eq!(log(r, 5).unwrap()[0].subject, "first");
        assert!(root(&std::env::temp_dir().join("definitely-not-a-repo-oryn")).is_none());
    }
}
