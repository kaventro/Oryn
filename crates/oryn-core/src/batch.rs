use parking_lot::Mutex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::services::fs_delete::delete_path;
use crate::services::fs_size::path_size;
use crate::services::fs_transfer::sink::ProgressSink;
use crate::services::fs_transfer::{copy, move_path, OverwritePolicy, TransferOut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchKind {
    Copy,
    Move,
    Trash,
    Delete,
    Compress,
    Extract,
}

impl BatchKind {
    pub fn verb(self) -> &'static str {
        match self {
            BatchKind::Copy => "Copying",
            BatchKind::Move => "Moving",
            BatchKind::Trash => "Moving to Trash",
            BatchKind::Delete => "Deleting",
            BatchKind::Compress => "Compressing",
            BatchKind::Extract => "Extracting",
        }
    }

    pub fn past(self) -> &'static str {
        match self {
            BatchKind::Copy => "Copied",
            BatchKind::Move => "Moved",
            BatchKind::Trash => "Moved to Trash",
            BatchKind::Delete => "Deleted",
            BatchKind::Compress => "Compressed",
            BatchKind::Extract => "Extracted",
        }
    }
}

#[derive(Default)]
pub struct Progress {
    pub total: AtomicU64,
    pub done: AtomicU64,
    pub bytes_total: AtomicU64,
    bytes_finished: AtomicU64,
    bytes_file: AtomicU64,
    file_total: AtomicU64,
    pub current: Mutex<String>,
    abort: AtomicBool,
}

impl Progress {
    pub fn cancel(&self) {
        self.abort.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }

    pub fn bytes_done(&self) -> u64 {
        self.bytes_finished.load(Ordering::Relaxed) + self.bytes_file.load(Ordering::Relaxed)
    }

    pub fn fraction(&self) -> f32 {
        let bytes = self.bytes_total.load(Ordering::Relaxed);
        if bytes > 0 {
            return (self.bytes_done() as f32 / bytes as f32).min(1.0);
        }
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        (self.done.load(Ordering::Relaxed) as f32 / total as f32).min(1.0)
    }

    fn set_current(&self, name: &str) {
        *self.current.lock() = name.to_string();
    }
}

impl ProgressSink for Progress {
    fn emit(&self, payload: Value) {
        let kind = payload.get("type").and_then(Value::as_str);
        if kind == Some("file") {
            let get = |k| payload.get(k).and_then(Value::as_u64).unwrap_or(0);
            self.bytes_file.store(get("bytes"), Ordering::Relaxed);
            self.file_total.store(get("total"), Ordering::Relaxed);
        }
        if matches!(kind, Some("done") | Some("skip")) {
            self.done.fetch_add(1, Ordering::Relaxed);
            let finished = self.file_total.swap(0, Ordering::Relaxed);
            self.bytes_file.store(0, Ordering::Relaxed);
            self.bytes_finished.fetch_add(finished, Ordering::Relaxed);
        }
        if let Some(path) = payload.get("path").and_then(Value::as_str) {
            self.set_current(path);
        }
    }

    fn is_aborted(&self) -> bool {
        self.is_cancelled()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BatchReport {
    pub done: usize,
    pub skipped: u64,
    pub failed: Vec<(String, String)>,
    pub cancelled: bool,
}

impl BatchReport {
    pub fn summary(&self, kind: BatchKind) -> String {
        let mut text = if self.cancelled {
            format!("Cancelled after {} item(s).", self.done)
        } else {
            format!("{} {} item(s).", kind.past(), self.done)
        };
        if self.skipped > 0 {
            text.push_str(&format!(" Skipped {} existing.", self.skipped));
        }
        if let Some((name, err)) = self.failed.first() {
            text.push_str(&format!(" {} failed: {name}: {err}", self.failed.len()));
        }
        text
    }
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn conflicts(sources: &[PathBuf], dest_dir: &Path) -> Vec<String> {
    sources
        .iter()
        .map(|s| name_of(s))
        .filter(|n| dest_dir.join(n).symlink_metadata().is_ok())
        .collect()
}

pub fn same_place(sources: &[PathBuf], dest_dir: &Path) -> bool {
    sources.iter().all(|s| s.parent() == Some(dest_dir))
}

pub fn run(
    kind: BatchKind,
    sources: &[PathBuf],
    dest_dir: Option<&Path>,
    policy: OverwritePolicy,
    progress: &Progress,
) -> BatchReport {
    if kind == BatchKind::Compress {
        return compress(sources, dest_dir, progress);
    }
    let summaries: Vec<_> = sources.iter().map(|s| path_size(s)).collect();
    let sizes: Vec<u64> = summaries.iter().map(|s| s.files.max(1)).collect();
    if matches!(kind, BatchKind::Copy | BatchKind::Move) {
        progress
            .bytes_total
            .store(summaries.iter().map(|s| s.bytes).sum(), Ordering::Relaxed);
    }
    progress
        .total
        .store(sizes.iter().sum::<u64>().max(1), Ordering::Relaxed);
    let mut report = BatchReport::default();
    for ((src, files), summary) in sources.iter().zip(sizes).zip(&summaries) {
        let summary_bytes = summary.bytes;
        let bytes_before = progress.bytes_finished.load(Ordering::Relaxed);
        if progress.is_cancelled() {
            report.cancelled = true;
            break;
        }
        let name = name_of(src);
        progress.set_current(&name);
        let before = progress.done.load(Ordering::Relaxed);
        let result: Result<Option<TransferOut>, String> = match (kind, dest_dir) {
            (BatchKind::Copy | BatchKind::Move, Some(dir)) => {
                let dst = dir.join(&name);
                if dst.starts_with(src) {
                    Err("can't put a folder inside itself".into())
                } else if kind == BatchKind::Copy {
                    Ok(Some(copy(progress, src, &dst, policy)))
                } else {
                    Ok(Some(move_path(progress, src, &dst, policy)))
                }
            }
            (BatchKind::Copy | BatchKind::Move | BatchKind::Extract, None) => {
                Err("no destination".into())
            }
            (BatchKind::Extract, Some(dir)) => {
                let out = extract_target(src, dir);
                crate::services::fs_archive::extract_archive(src, &out)
                    .map(|_| None)
                    .map_err(|e| e.to_string())
            }
            (BatchKind::Compress, _) => Ok(None),
            (BatchKind::Trash | BatchKind::Delete, _) => {
                delete_path(&src.to_string_lossy(), kind == BatchKind::Trash, None)
                    .map(|_| None)
                    .map_err(|e| e.to_string())
            }
        };
        match result {
            Ok(Some(out)) if out.cancelled == Some(true) => {
                report.cancelled = true;
                break;
            }
            Ok(Some(out)) if !out.ok => report
                .failed
                .push((name, out.error.unwrap_or_else(|| "failed".into()))),
            Ok(Some(out)) => {
                report.skipped += out.skipped.unwrap_or(0);
                if out.src_kept != Some(true) {
                    report.done += 1;
                }
            }
            Ok(None) => report.done += 1,
            Err(err) => report.failed.push((name, err)),
        }
        let target = before + files;
        progress.done.fetch_max(target, Ordering::Relaxed);
        progress.bytes_file.store(0, Ordering::Relaxed);
        progress
            .bytes_finished
            .fetch_max(bytes_before + summary_bytes, Ordering::Relaxed);
    }
    report
}

pub fn is_archive(path: &Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    [".zip", ".tar", ".tar.gz", ".tgz"]
        .iter()
        .any(|e| name.ends_with(e))
}

fn unique(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{stem}{ext}")
        } else {
            format!("{stem} {n}{ext}")
        };
        let p = dir.join(name);
        if p.symlink_metadata().is_err() {
            return p;
        }
        n += 1;
    }
}

fn archive_stem(path: &Path) -> String {
    let name = name_of(path);
    let lower = name.to_lowercase();
    for ext in [".tar.gz", ".tgz", ".tar", ".zip"] {
        if lower.ends_with(ext) {
            return name[..name.len() - ext.len()].to_string();
        }
    }
    name
}

pub fn extract_target(archive: &Path, dir: &Path) -> PathBuf {
    unique(dir, &archive_stem(archive), "")
}

pub fn archive_target(sources: &[PathBuf], dir: &Path) -> PathBuf {
    let stem = match sources {
        [one] => {
            let name = name_of(one);
            if one.is_dir() {
                name
            } else {
                match name.rfind('.') {
                    Some(i) if i > 0 => name[..i].to_string(),
                    _ => name,
                }
            }
        }
        _ => "Archive".to_string(),
    };
    unique(dir, &stem, ".zip")
}

fn compress(sources: &[PathBuf], dest_dir: Option<&Path>, progress: &Progress) -> BatchReport {
    let mut report = BatchReport::default();
    progress.total.store(1, Ordering::Relaxed);
    let Some(dir) = dest_dir.or_else(|| sources.first().and_then(|s| s.parent())) else {
        report
            .failed
            .push(("archive".into(), "no destination".into()));
        return report;
    };
    let target = archive_target(sources, dir);
    progress.set_current(&name_of(&target));
    match crate::services::fs_archive::compress_zip(sources, &target) {
        Ok(()) => report.done = sources.len(),
        Err(e) => {
            let _ = std::fs::remove_file(&target);
            report.failed.push((name_of(&target), e.to_string()));
        }
    }
    progress.done.store(1, Ordering::Relaxed);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("folder/inner")).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("folder/b.txt"), "bb").unwrap();
        fs::write(src.join("folder/inner/c.txt"), "ccc").unwrap();
        (tmp, src, dst)
    }

    #[test]
    fn copies_with_progress() {
        let (_tmp, src, dst) = tree();
        let p = Progress::default();
        let items = vec![src.join("a.txt"), src.join("folder")];
        let report = run(
            BatchKind::Copy,
            &items,
            Some(&dst),
            OverwritePolicy::Overwrite,
            &p,
        );
        assert_eq!(report.done, 2);
        assert!(report.failed.is_empty());
        assert_eq!(
            fs::read_to_string(dst.join("folder/inner/c.txt")).unwrap(),
            "ccc"
        );
        assert_eq!(p.total.load(Ordering::Relaxed), 3);
        assert_eq!(p.fraction(), 1.0);
        assert!(src.join("a.txt").exists());
        assert_eq!(report.summary(BatchKind::Copy), "Copied 2 item(s).");
    }

    #[test]
    fn moves_and_reports_conflicts() {
        let (_tmp, src, dst) = tree();
        fs::write(dst.join("a.txt"), "old").unwrap();
        let items = vec![src.join("a.txt"), src.join("folder")];
        assert_eq!(conflicts(&items, &dst), vec!["a.txt".to_string()]);
        assert!(!same_place(&items, &dst));
        assert!(same_place(&items, &src));
        let p = Progress::default();
        let report = run(
            BatchKind::Move,
            &items,
            Some(&dst),
            OverwritePolicy::Skip,
            &p,
        );
        assert_eq!(report.done, 1);
        assert_eq!(report.skipped, 1);
        assert!(src.join("a.txt").exists());
        assert!(!src.join("folder").exists());
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "old");
        assert!(report
            .summary(BatchKind::Move)
            .contains("Skipped 1 existing"));
    }

    #[test]
    fn refuses_folder_into_itself() {
        let (_tmp, src, _dst) = tree();
        let p = Progress::default();
        let items = vec![src.join("folder")];
        let inside = src.join("folder/inner");
        let report = run(
            BatchKind::Copy,
            &items,
            Some(&inside),
            OverwritePolicy::Overwrite,
            &p,
        );
        assert_eq!(report.done, 0);
        assert_eq!(report.failed.len(), 1);
        assert!(report.summary(BatchKind::Copy).contains("1 failed: folder"));
        let none = run(
            BatchKind::Move,
            &items,
            None,
            OverwritePolicy::Overwrite,
            &p,
        );
        assert_eq!(none.failed[0].1, "no destination");
    }

    #[test]
    fn deletes_permanently() {
        let (_tmp, src, _dst) = tree();
        let p = Progress::default();
        let items = vec![src.join("a.txt"), src.join("folder")];
        let report = run(
            BatchKind::Delete,
            &items,
            None,
            OverwritePolicy::Overwrite,
            &p,
        );
        assert_eq!(report.done, 2);
        assert!(!src.join("folder").exists());
        assert_eq!(BatchKind::Delete.verb(), "Deleting");
        assert_eq!(BatchKind::Trash.past(), "Moved to Trash");
        assert_eq!(BatchKind::Copy.verb(), "Copying");
        assert_eq!(BatchKind::Move.verb(), "Moving");
        assert_eq!(BatchKind::Trash.verb(), "Moving to Trash");
    }

    #[test]
    fn cancel_stops_before_next_item() {
        let (_tmp, src, dst) = tree();
        let p = Progress::default();
        p.cancel();
        assert!(p.is_aborted());
        let report = run(
            BatchKind::Copy,
            &[src.join("a.txt")],
            Some(&dst),
            OverwritePolicy::Overwrite,
            &p,
        );
        assert!(report.cancelled);
        assert_eq!(
            report.summary(BatchKind::Copy),
            "Cancelled after 0 item(s)."
        );
        assert_eq!(Progress::default().fraction(), 0.0);
    }

    #[test]
    fn sink_counts_done_and_skip() {
        let p = Progress::default();
        p.emit(serde_json::json!({"type": "done", "path": "x"}));
        p.emit(serde_json::json!({"type": "skip", "path": "y"}));
        p.emit(serde_json::json!({"type": "file", "path": "z"}));
        assert_eq!(p.done.load(Ordering::Relaxed), 2);
        p.bytes_total.store(100, Ordering::Relaxed);
        p.emit(serde_json::json!({"type": "file", "path": "z", "bytes": 25, "total": 50}));
        assert_eq!(p.bytes_done(), 25);
        assert_eq!(p.fraction(), 0.25);
        p.emit(serde_json::json!({"type": "done", "path": "z"}));
        assert_eq!(p.bytes_done(), 50);
        assert_eq!(*p.current.lock(), "z");
    }

    #[test]
    fn compress_then_extract_round_trips() {
        let (_tmp, src, dst) = tree();
        let p = Progress::default();
        let sources = vec![src.join("a.txt"), src.join("folder")];
        let r = run(
            BatchKind::Compress,
            &sources,
            Some(&dst),
            OverwritePolicy::Skip,
            &p,
        );
        assert_eq!(r.done, 2);
        let zip = dst.join("Archive.zip");
        assert!(zip.is_file());
        assert_eq!(archive_target(&sources, &dst), dst.join("Archive 2.zip"));
        assert!(is_archive(&zip));
        let r = run(
            BatchKind::Extract,
            std::slice::from_ref(&zip),
            Some(&dst),
            OverwritePolicy::Skip,
            &p,
        );
        assert_eq!(r.done, 1, "{:?}", r.failed);
        let out = dst.join("Archive");
        assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "a");
        assert_eq!(
            fs::read_to_string(out.join("folder/inner/c.txt")).unwrap(),
            "ccc"
        );
        assert_eq!(extract_target(&zip, &dst), dst.join("Archive 2"));
        assert_eq!(
            archive_target(&[src.join("a.txt")], &dst),
            dst.join("a.zip")
        );
    }
}
