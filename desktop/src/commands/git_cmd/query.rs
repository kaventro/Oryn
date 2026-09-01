use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;

use super::shared::{git, DirPathIn, GitBlameIn, GitDiffIn, GitLogIn, RepoPathIn};

pub async fn git_is_repo(input: DirPathIn) -> Result<Value, String> {
    let mut cur = std::path::PathBuf::from(&input.dir_path);
    loop {
        let git_marker = cur.join(".git");
        if git_marker.exists() {
            let root = cur.to_string_lossy().to_string();
            return Ok(json!({"ok": true, "root": root}));
        }
        if !cur.pop() {
            break;
        }
    }
    Ok(json!({"ok": false}))
}

pub async fn git_status(input: RepoPathIn) -> Result<Value, String> {
    let repo_path = &input.repo_path;
    let o = git(&["status", "--porcelain=v2", "--branch", "-u"], repo_path)
        .await
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Ok(json!({"ok": false, "error": "not a git repo"}));
    }

    let stdout = String::from_utf8_lossy(&o.stdout);
    let mut branch = "HEAD".to_string();
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut files = Vec::new();

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(a) = parts[0].strip_prefix('+') {
                    ahead = a.parse().unwrap_or(0);
                }
                if let Some(b) = parts[1].strip_prefix('-') {
                    behind = b.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") {
            // Ordinary modified / staged file: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            let parts: Vec<&str> = line.splitn(9, ' ').collect();
            if parts.len() >= 9 {
                let xy = parts[1];
                let file = parts[8];
                files.push(json!({
                    "xy": xy,
                    "file": file,
                    "index": xy.chars().next().map(|c| c.to_string()).unwrap_or_default(),
                    "worktree": xy.chars().nth(1).map(|c| c.to_string()).unwrap_or_default(),
                }));
            }
        } else if line.starts_with("2 ") {
            // Renamed / copied file
            let parts: Vec<&str> = line.splitn(10, ' ').collect();
            if parts.len() >= 10 {
                let xy = parts[1];
                let path_part = parts[9];
                let file = path_part.split('\t').next().unwrap_or(path_part);
                files.push(json!({
                    "xy": xy,
                    "file": file,
                    "index": xy.chars().next().map(|c| c.to_string()).unwrap_or_default(),
                    "worktree": xy.chars().nth(1).map(|c| c.to_string()).unwrap_or_default(),
                }));
            }
        } else if line.starts_with("u ") {
            // Unmerged conflict
            let parts: Vec<&str> = line.splitn(11, ' ').collect();
            if parts.len() >= 11 {
                let xy = parts[1];
                let file = parts[10];
                files.push(json!({
                    "xy": xy,
                    "file": file,
                    "index": "U",
                    "worktree": "U",
                }));
            }
        } else if let Some(file) = line.strip_prefix("? ") {
            files.push(json!({
                "xy": "??",
                "file": file.trim(),
                "index": "?",
                "worktree": "?",
            }));
        }
    }

    Ok(json!({
        "ok": true,
        "branch": branch,
        "files": files,
        "ahead": ahead,
        "behind": behind
    }))
}

pub async fn git_log(input: GitLogIn) -> Result<Value, String> {
    let n = input.max_count.unwrap_or(50);
    let fmt = "%H%x1f%h%x1f%an%x1f%ae%x1f%ai%x1f%s";
    let mut args: Vec<String> = vec![
        "log".into(),
        format!("--max-count={}", n),
        format!("--format={}", fmt),
    ];
    if let Some(f) = input.file_path {
        let mut rel = f.trim().to_string();
        if rel.starts_with(&input.repo_path) {
            rel = rel[input.repo_path.len()..]
                .trim_start_matches(['/', '\\'])
                .to_string();
        } else {
            rel = rel.trim_start_matches(['/', '\\']).to_string();
        }
        if !rel.is_empty() {
            args.push("--follow".into());
            args.push("--".into());
            args.push(rel);
        }
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = git(&args_ref, &input.repo_path)
        .await
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Ok(
            json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string(), "commits": []}),
        );
    }

    let commits: Vec<Value> = String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let mut parts = line.split('\x1f');
            let hash = parts.next()?.to_string();
            let short = parts.next()?.to_string();
            let author = parts.next()?.to_string();
            let email = parts.next()?.to_string();
            let date = parts.next()?.to_string();
            let subject = parts.collect::<Vec<_>>().join("\x1f");
            Some(json!({
                "hash": hash,
                "short": short,
                "author": author,
                "email": email,
                "date": date,
                "subject": subject
            }))
        })
        .collect();

    Ok(json!({"ok": true, "commits": commits}))
}

pub async fn git_diff(input: GitDiffIn) -> Result<Value, String> {
    let mut args: Vec<String> = vec!["diff".into(), "--unified=5".into()];
    if let Some(r) = input.ref1 {
        if !r.is_empty() {
            if r.starts_with('-') {
                return Ok(json!({"ok": false, "error": "invalid git ref", "diff": ""}));
            }
            args.push(r);
        }
    }
    if let Some(r) = input.ref2 {
        if !r.is_empty() {
            if r.starts_with('-') {
                return Ok(json!({"ok": false, "error": "invalid git ref", "diff": ""}));
            }
            args.push(r);
        }
    }
    if let Some(f) = input.file_path {
        let mut rel = f.trim().to_string();
        if rel.starts_with(&input.repo_path) {
            rel = rel[input.repo_path.len()..]
                .trim_start_matches(['/', '\\'])
                .to_string();
        } else {
            rel = rel.trim_start_matches(['/', '\\']).to_string();
        }
        if !rel.is_empty() {
            args.push("--".into());
            args.push(rel);
        }
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = Command::new("git")
        .args(&args_ref)
        .current_dir(&input.repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
    if o.status.success() || o.status.code() == Some(1) {
        return Ok(json!({"ok": true, "diff": stdout}));
    }

    Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string(), "diff": ""}))
}

pub async fn git_branches(input: RepoPathIn) -> Result<Value, String> {
    let o = git(
        &[
            "branch",
            "-a",
            "--format=%(refname:short)%x1f%(HEAD)%x1f%(upstream:short)",
        ],
        &input.repo_path,
    )
    .await
    .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Ok(
            json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string(), "branches": []}),
        );
    }

    let branches: Vec<Value> = String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let p: Vec<&str> = line.split('\x1f').collect();
            let name = p.first().copied().unwrap_or("");
            let head = p.get(1).copied().unwrap_or("");
            let upstream = p.get(2).copied().unwrap_or("");
            json!({
                "name": name,
                "isCurrent": head == "*",
                "upstream": upstream
            })
        })
        .collect();

    Ok(json!({"ok": true, "branches": branches}))
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitBlameLine {
    pub line_num: u32,
    pub commit_hash: String,
    pub commit_short: String,
    pub author: String,
    pub author_mail: String,
    pub author_time: i64,
    pub summary: String,
    pub content: String,
}

pub fn parse_git_blame_porcelain(raw: &str) -> Vec<GitBlameLine> {
    let mut result = Vec::new();
    let mut current_hash = String::new();
    let mut current_line_num: u32 = 0;
    let mut current_author = String::new();
    let mut current_author_mail = String::new();
    let mut current_author_time: i64 = 0;
    let mut current_summary = String::new();

    for line in raw.lines() {
        if let Some(content) = line.strip_prefix('\t') {
            let short = if current_hash.len() >= 8 {
                current_hash[..8].to_string()
            } else {
                current_hash.clone()
            };
            result.push(GitBlameLine {
                line_num: current_line_num,
                commit_hash: current_hash.clone(),
                commit_short: short,
                author: current_author.clone(),
                author_mail: current_author_mail.clone(),
                author_time: current_author_time,
                summary: current_summary.clone(),
                content: content.to_string(),
            });
        } else if let Some(author) = line.strip_prefix("author ") {
            current_author = author.to_string();
        } else if let Some(mail) = line.strip_prefix("author-mail ") {
            current_author_mail = mail.trim_matches(|c| c == '<' || c == '>').to_string();
        } else if let Some(time_str) = line.strip_prefix("author-time ") {
            current_author_time = time_str.trim().parse::<i64>().unwrap_or(0);
        } else if let Some(summary) = line.strip_prefix("summary ") {
            current_summary = summary.to_string();
        } else {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3
                && parts[0].len() == 40
                && parts[0].chars().all(|c| c.is_ascii_hexdigit())
            {
                current_hash = parts[0].to_string();
                if let Ok(ln) = parts[2].parse::<u32>() {
                    current_line_num = ln;
                }
            }
        }
    }
    result
}

pub async fn git_blame(input: GitBlameIn) -> Result<Value, String> {
    let mut file_path = input.file_path.trim().to_string();
    if file_path.starts_with(&input.repo_path) {
        file_path = file_path[input.repo_path.len()..]
            .trim_start_matches(['/', '\\'])
            .to_string();
    } else {
        file_path = file_path.trim_start_matches(['/', '\\']).to_string();
    }

    let mut args: Vec<String> = vec!["blame".into(), "--line-porcelain".into()];
    if let Some(r) = input.git_ref {
        let r = r.trim();
        if !r.is_empty() {
            if r.starts_with('-') {
                return Ok(json!({"ok": false, "error": "invalid git ref", "lines": []}));
            }
            args.push(r.to_string());
        }
    }
    args.push("--".into());
    args.push(file_path);

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = Command::new("git")
        .args(&args_ref)
        .current_dir(&input.repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Ok(json!({
            "ok": false,
            "error": String::from_utf8_lossy(&o.stderr).to_string(),
            "lines": []
        }));
    }

    let stdout = String::from_utf8_lossy(&o.stdout);
    let lines = parse_git_blame_porcelain(&stdout);

    Ok(json!({
        "ok": true,
        "lines": lines
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_porcelain_blame() {
        let sample = "\
deadbeefcafebabe0123456789abcdef01234567 1 1 2
author Alice Smith
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice Smith
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary Initial commit
filename test.txt
\thello world
deadbeefcafebabe0123456789abcdef01234567 2 2
author Alice Smith
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice Smith
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary Initial commit
filename test.txt
\tsecond line";

        let lines = parse_git_blame_porcelain(sample);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line_num, 1);
        assert_eq!(lines[0].commit_short, "deadbeef");
        assert_eq!(lines[0].author, "Alice Smith");
        assert_eq!(lines[0].author_mail, "alice@example.com");
        assert_eq!(lines[0].author_time, 1700000000);
        assert_eq!(lines[0].summary, "Initial commit");
        assert_eq!(lines[0].content, "hello world");

        assert_eq!(lines[1].line_num, 2);
        assert_eq!(lines[1].content, "second line");
    }
}
