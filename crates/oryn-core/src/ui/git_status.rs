use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const CACHE_TTL: Duration = Duration::from_millis(2500);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFileEntry {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub index: Option<char>,
    #[serde(default)]
    pub worktree: Option<char>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPaneInfo {
    pub root: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSnapshot {
    pub git: Option<GitPaneInfo>,
    pub files: Vec<GitFileEntry>,
}

pub fn tag_from_entry(entry: &GitFileEntry) -> char {
    let index = entry.index.unwrap_or(' ');
    let worktree = entry.worktree.unwrap_or(' ');
    if index != ' ' && index != '?' {
        return if index == 'A' { 'A' } else { 'M' };
    }
    match worktree {
        '?' => '?',
        'D' => 'D',
        _ => 'M',
    }
}

pub fn statuses_for_dir(dir_path: &str, snapshot: &GitSnapshot) -> HashMap<String, char> {
    let mut out = HashMap::new();
    let Some(git) = &snapshot.git else {
        return out;
    };
    let rel = dir_path
        .strip_prefix(git.root.as_str())
        .map(|r| r.trim_start_matches(['/', '\\']))
        .unwrap_or("");
    let prefix = if rel.is_empty() {
        String::new()
    } else {
        format!("{}/", rel.trim_end_matches(['/', '\\']))
    };
    for entry in &snapshot.files {
        if entry.file.is_empty() {
            continue;
        }
        let path = if prefix.is_empty() {
            entry.file.as_str()
        } else {
            match entry.file.strip_prefix(prefix.as_str()) {
                Some(rest) => rest,
                None => continue,
            }
        };
        let base = path.split(['/', '\\']).next().unwrap_or("");
        if base.is_empty() {
            continue;
        }
        out.insert(base.to_string(), tag_from_entry(entry));
    }
    out
}

#[derive(Default)]
pub struct SnapshotCache {
    entries: HashMap<String, (Instant, GitSnapshot)>,
}

impl SnapshotCache {
    pub fn get(&self, root: &str, now: Instant) -> Option<&GitSnapshot> {
        self.entries
            .get(root)
            .filter(|(at, _)| now.duration_since(*at) < CACHE_TTL)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn insert(&mut self, root: String, snapshot: GitSnapshot, now: Instant) {
        self.entries.insert(root, (now, snapshot));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(file: &str, index: char, worktree: char) -> GitFileEntry {
        GitFileEntry {
            file: file.into(),
            index: Some(index),
            worktree: Some(worktree),
        }
    }

    fn repo(files: Vec<GitFileEntry>) -> GitSnapshot {
        GitSnapshot {
            git: Some(GitPaneInfo {
                root: "/repo".into(),
                branch: "main".into(),
                ahead: 0,
                behind: 0,
            }),
            files,
        }
    }

    #[test]
    fn maps_staged_untracked_modified_and_deleted() {
        assert_eq!(tag_from_entry(&entry("", 'A', ' ')), 'A');
        assert_eq!(tag_from_entry(&entry("", 'M', ' ')), 'M');
        assert_eq!(tag_from_entry(&entry("", ' ', '?')), '?');
        assert_eq!(tag_from_entry(&entry("", '?', '?')), '?');
        assert_eq!(tag_from_entry(&entry("", ' ', 'D')), 'D');
        assert_eq!(tag_from_entry(&entry("", 'D', ' ')), 'M');
    }

    #[test]
    fn badges_nested_files_onto_the_parent_folder_at_repo_root() {
        let map = statuses_for_dir(
            "/repo",
            &repo(vec![
                entry("src/app.ts", ' ', 'M'),
                entry("README.md", 'A', ' '),
            ]),
        );
        assert_eq!(map.get("src"), Some(&'M'));
        assert_eq!(map.get("README.md"), Some(&'A'));
        assert_eq!(map.get("clean.txt"), None);
    }

    #[test]
    fn only_maps_files_under_the_current_directory() {
        let map = statuses_for_dir(
            "/repo/ios",
            &repo(vec![
                entry("README.md", ' ', 'M'),
                entry("ios/App.swift", ' ', '?'),
            ]),
        );
        assert_eq!(map.get("App.swift"), Some(&'?'));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn no_repo_means_no_badges() {
        assert!(statuses_for_dir("/x", &GitSnapshot::default()).is_empty());
    }

    #[test]
    fn cache_expires_after_the_ttl() {
        let mut cache = SnapshotCache::default();
        let now = Instant::now();
        cache.insert("/repo".into(), repo(vec![]), now);
        assert!(cache
            .get("/repo", now + Duration::from_millis(100))
            .is_some());
        assert!(cache.get("/repo", now + CACHE_TTL).is_none());
        cache.clear();
        assert!(cache.get("/repo", now).is_none());
    }
}
