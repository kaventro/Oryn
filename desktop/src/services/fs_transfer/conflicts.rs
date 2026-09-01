use std::path::Path;

use walkdir::WalkDir;

/// Items in the source that already exist at the mirrored destination path.
/// Informs the UI's one-shot Overwrite/Skip dialog before a transfer starts.
#[derive(Debug, Default)]
pub struct ConflictReport {
    pub conflicts: Vec<String>,
    pub truncated: bool,
}

pub fn scan(src: &Path, dst: &Path, limit: usize) -> ConflictReport {
    let mut report = ConflictReport::default();

    let Ok(src_meta) = std::fs::symlink_metadata(src) else {
        return report;
    };

    if !src_meta.is_dir() {
        if std::fs::symlink_metadata(dst).is_ok() {
            report
                .conflicts
                .push(file_name_of(src).unwrap_or_else(|| src.to_string_lossy().to_string()));
        }
        return report;
    }

    for entry in WalkDir::new(src)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            continue; // directories merge; only leaf entries clash
        }
        let Ok(rel) = entry.path().strip_prefix(src) else {
            continue;
        };
        if std::fs::symlink_metadata(dst.join(rel)).is_ok() {
            if report.conflicts.len() >= limit {
                report.truncated = true;
                break;
            }
            report
                .conflicts
                .push(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    report
}

fn file_name_of(path: &Path) -> Option<String> {
    path.file_name().map(|s| s.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::scan;
    use std::fs;

    #[test]
    fn reports_clashing_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::create_dir_all(dst.join("sub")).unwrap();
        fs::write(src.join("clash.txt"), b"a").unwrap();
        fs::write(src.join("fresh.txt"), b"b").unwrap();
        fs::write(src.join("sub/deep.txt"), b"c").unwrap();
        fs::write(dst.join("clash.txt"), b"old").unwrap();
        fs::write(dst.join("sub/deep.txt"), b"old").unwrap();

        let mut report = scan(&src, &dst, 50);
        report.conflicts.sort();

        assert_eq!(report.conflicts, vec!["clash.txt", "sub/deep.txt"]);
        assert!(!report.truncated);
    }

    #[test]
    fn single_file_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("f.txt");
        let dst = tmp.path().join("dst-f.txt");
        fs::write(&src, b"x").unwrap();
        fs::write(&dst, b"y").unwrap();

        let report = scan(&src, &dst, 50);
        assert_eq!(report.conflicts, vec!["f.txt"]);
    }

    #[test]
    fn no_conflicts_when_target_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("a.txt"), b"1").unwrap();

        let report = scan(&src, &tmp.path().join("dst"), 50);
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn truncates_at_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        for i in 0..10 {
            fs::write(src.join(format!("f{i}.txt")), b"n").unwrap();
            fs::write(dst.join(format!("f{i}.txt")), b"o").unwrap();
        }

        let report = scan(&src, &dst, 3);
        assert_eq!(report.conflicts.len(), 3);
        assert!(report.truncated);
    }
}
