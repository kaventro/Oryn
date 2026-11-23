use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compare::shared::{binary_differ, build_dir_map, same_inode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Same,
    LeftNewer,
    RightNewer,
    Differs,
    LeftOnly,
    RightOnly,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Same => "Same",
            State::LeftNewer => "Left is newer",
            State::RightNewer => "Right is newer",
            State::Differs => "Different",
            State::LeftOnly => "Only on the left",
            State::RightOnly => "Only on the right",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub rel: String,
    pub is_dir: bool,
    pub state: State,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    Both,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Copy { from: PathBuf, to: PathBuf },
    Remove(PathBuf),
}

pub fn compare(left: &Path, right: &Path, by_content: bool) -> Result<Vec<Row>, String> {
    let l = build_dir_map(left)?;
    let r = build_dir_map(right)?;
    let keys: BTreeSet<&String> = l.keys().chain(r.keys()).collect();
    let mut rows = Vec::with_capacity(keys.len());
    for key in keys {
        let row = match (l.get(key), r.get(key)) {
            (Some(a), None) => Row {
                rel: key.clone(),
                is_dir: a.is_dir,
                state: State::LeftOnly,
                left_size: Some(a.size),
                right_size: None,
            },
            (None, Some(b)) => Row {
                rel: key.clone(),
                is_dir: b.is_dir,
                state: State::RightOnly,
                left_size: None,
                right_size: Some(b.size),
            },
            (Some(a), Some(b)) => {
                let state = if a.is_dir && b.is_dir || same_inode(a, b) {
                    State::Same
                } else if a.is_dir != b.is_dir || a.size != b.size {
                    newer(a.mtime, b.mtime)
                } else if by_content {
                    if binary_differ(Path::new(&a.full), Path::new(&b.full))? {
                        newer(a.mtime, b.mtime)
                    } else {
                        State::Same
                    }
                } else if (a.mtime - b.mtime).abs() < 2000.0 {
                    State::Same
                } else {
                    newer(a.mtime, b.mtime)
                };
                Row {
                    rel: key.clone(),
                    is_dir: a.is_dir && b.is_dir,
                    state,
                    left_size: Some(a.size),
                    right_size: Some(b.size),
                }
            }
            (None, None) => continue,
        };
        rows.push(row);
    }
    Ok(rows)
}

fn newer(l: f64, r: f64) -> State {
    if (l - r).abs() < 2000.0 {
        State::Differs
    } else if l > r {
        State::LeftNewer
    } else {
        State::RightNewer
    }
}

fn covered(rel: &str, dirs: &BTreeSet<String>) -> bool {
    let mut cur = rel;
    while let Some(i) = cur.rfind('/') {
        cur = &cur[..i];
        if dirs.contains(cur) {
            return true;
        }
    }
    false
}

pub fn plan(rows: &[Row], left: &Path, right: &Path, dir: Direction, mirror: bool) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut copied_dirs = BTreeSet::new();
    let mut removed_dirs = BTreeSet::new();
    let copy = |rel: &str, to_right: bool| {
        let (a, b) = if to_right {
            (left, right)
        } else {
            (right, left)
        };
        Step::Copy {
            from: a.join(rel),
            to: b.join(rel),
        }
    };
    for row in rows {
        if covered(&row.rel, &copied_dirs) || covered(&row.rel, &removed_dirs) {
            continue;
        }
        let to_right = match (row.state, dir) {
            (State::Same, _) => continue,
            (State::LeftOnly, Direction::LeftToRight | Direction::Both) => Some(true),
            (State::RightOnly, Direction::RightToLeft | Direction::Both) => Some(false),
            (State::LeftNewer, Direction::LeftToRight | Direction::Both) => Some(true),
            (State::RightNewer, Direction::RightToLeft | Direction::Both) => Some(false),
            (State::LeftNewer | State::Differs, Direction::RightToLeft) => Some(false),
            (State::RightNewer | State::Differs, Direction::LeftToRight) => Some(true),
            (State::Differs, Direction::Both) => continue,
            (State::RightOnly, Direction::LeftToRight) => {
                if mirror {
                    removed_dirs.insert(row.rel.clone());
                    steps.push(Step::Remove(right.join(&row.rel)));
                }
                continue;
            }
            (State::LeftOnly, Direction::RightToLeft) => {
                if mirror {
                    removed_dirs.insert(row.rel.clone());
                    steps.push(Step::Remove(left.join(&row.rel)));
                }
                continue;
            }
        };
        let Some(to_right) = to_right else { continue };
        if row.is_dir {
            copied_dirs.insert(row.rel.clone());
        }
        steps.push(copy(&row.rel, to_right));
    }
    steps
}

pub fn apply(steps: &[Step]) -> Vec<(String, String)> {
    let mut failed = Vec::new();
    for step in steps {
        let res = match step {
            Step::Remove(p) => {
                if p.is_dir() {
                    fs::remove_dir_all(p)
                } else {
                    fs::remove_file(p)
                }
            }
            Step::Copy { from, to } => copy_tree(from, to),
        };
        if let Err(e) = res {
            let p = match step {
                Step::Remove(p) => p,
                Step::Copy { to, .. } => to,
            };
            failed.push((p.display().to_string(), e.to_string()));
        }
    }
    failed
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        if let Ok(meta) = fs::metadata(from) {
            if let Ok(m) = meta.modified() {
                let _ = filetime::set_file_mtime(to, filetime::FileTime::from_system_time(m));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let l = tmp.path().join("l");
        let r = tmp.path().join("r");
        fs::create_dir_all(l.join("only-left/deep")).unwrap();
        fs::create_dir_all(r.join("only-right")).unwrap();
        fs::write(l.join("only-left/deep/x.txt"), "x").unwrap();
        fs::write(r.join("only-right/y.txt"), "y").unwrap();
        fs::write(l.join("same.txt"), "same").unwrap();
        fs::write(r.join("same.txt"), "same").unwrap();
        fs::write(l.join("newer.txt"), "left wins").unwrap();
        fs::write(r.join("newer.txt"), "old").unwrap();
        let t = FileTime::from_unix_time(1_700_000_000, 0);
        set_file_mtime(l.join("same.txt"), t).unwrap();
        set_file_mtime(r.join("same.txt"), t).unwrap();
        set_file_mtime(r.join("newer.txt"), t).unwrap();
        set_file_mtime(
            l.join("newer.txt"),
            FileTime::from_unix_time(1_700_100_000, 0),
        )
        .unwrap();
        (tmp, l, r)
    }

    #[test]
    fn compares_and_syncs_left_to_right() {
        let (_tmp, l, r) = setup();
        let rows = compare(&l, &r, true).unwrap();
        let get = |rel: &str| rows.iter().find(|r| r.rel == rel).unwrap().state;
        assert_eq!(get("same.txt"), State::Same);
        assert_eq!(get("newer.txt"), State::LeftNewer);
        assert_eq!(get("only-left"), State::LeftOnly);
        assert_eq!(get("only-left/deep/x.txt"), State::LeftOnly);
        assert_eq!(get("only-right/y.txt"), State::RightOnly);

        let steps = plan(&rows, &l, &r, Direction::LeftToRight, false);
        assert_eq!(steps.len(), 2);
        assert!(apply(&steps).is_empty());
        assert_eq!(
            fs::read_to_string(r.join("newer.txt")).unwrap(),
            "left wins"
        );
        assert!(r.join("only-left/deep/x.txt").is_file());
        assert!(r.join("only-right").exists());

        let rows = compare(&l, &r, true).unwrap();
        let steps = plan(&rows, &l, &r, Direction::LeftToRight, true);
        assert_eq!(steps, vec![Step::Remove(r.join("only-right"))]);
        apply(&steps);
        let rows = compare(&l, &r, true).unwrap();
        assert!(rows.iter().all(|r| r.state == State::Same));
    }

    #[test]
    fn two_way_sync_takes_newest_and_both_sides_extras() {
        let (_tmp, l, r) = setup();
        let rows = compare(&l, &r, false).unwrap();
        let steps = plan(&rows, &l, &r, Direction::Both, false);
        assert_eq!(steps.len(), 3);
        apply(&steps);
        assert!(l.join("only-right/y.txt").is_file());
        assert!(r.join("only-left/deep/x.txt").is_file());
        assert_eq!(
            fs::read_to_string(r.join("newer.txt")).unwrap(),
            "left wins"
        );
    }
}
