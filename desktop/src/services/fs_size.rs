use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirSizeSummary {
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpaceNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub is_dir: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpaceAnalysis {
    pub ok: bool,
    pub path: String,
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub items: Vec<DiskSpaceNode>,
}

/// Computes recursive disk space breakdown of immediate items in a directory.
/// Safely skips symlinks and junctions, uses bounded thread pool to avoid OS starvation.
pub fn analyze_directory(root: &Path) -> std::io::Result<DiskSpaceAnalysis> {
    let mut items = Vec::new();
    let entries = std::fs::read_dir(root)?;

    let mut dir_entries = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let Ok(ft) = entry.file_type() else {
            continue;
        };

        // Symlinks & junctions are skipped to prevent cycles and cross-volume traversing
        if ft.is_symlink() {
            continue;
        }

        if ft.is_dir() {
            dir_entries.push((name, path));
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            items.push(DiskSpaceNode {
                name,
                path: path.to_string_lossy().to_string(),
                size,
                files: 1,
                dirs: 0,
                is_dir: false,
            });
        }
    }

    // Compute sizes of subdirectories using a bounded worker pool
    if !dir_entries.is_empty() {
        use std::sync::Mutex as StdMutex;
        let queue = StdMutex::new(dir_entries);
        let dir_nodes = StdMutex::new(Vec::new());

        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(8);

        std::thread::scope(|scope| {
            for _ in 0..workers {
                let queue = &queue;
                let dir_nodes = &dir_nodes;
                scope.spawn(move || loop {
                    let next = {
                        let mut guard = queue.lock().unwrap();
                        guard.pop()
                    };
                    let Some((name, path)) = next else {
                        break;
                    };
                    let summary = path_size(&path);
                    let mut guard = dir_nodes.lock().unwrap();
                    guard.push(DiskSpaceNode {
                        name,
                        path: path.to_string_lossy().to_string(),
                        size: summary.bytes,
                        files: summary.files,
                        dirs: summary.dirs,
                        is_dir: true,
                    });
                });
            }
        });

        let mut dirs_done = dir_nodes.into_inner().unwrap();
        items.append(&mut dirs_done);
    }

    // Sort descending by size
    items.sort_by(|a, b| b.size.cmp(&a.size));

    let mut total_size = 0u64;
    let mut total_files = 0u64;
    let mut total_dirs = 0u64;

    for it in &items {
        total_size += it.size;
        total_files += it.files;
        total_dirs += if it.is_dir { it.dirs + 1 } else { 0 };
    }

    Ok(DiskSpaceAnalysis {
        ok: true,
        path: root.to_string_lossy().to_string(),
        total_size,
        total_files,
        total_dirs,
        items,
    })
}

/// Size of a path: file length for files, recursive content size for dirs.
/// Symlinks are never followed (a cycle cannot hang the scan).
pub fn path_size(root: &Path) -> DirSizeSummary {
    let Ok(meta) = std::fs::symlink_metadata(root) else {
        return DirSizeSummary::default();
    };
    if !meta.is_dir() {
        return DirSizeSummary {
            bytes: meta.len(),
            files: 1,
            dirs: 0,
        };
    }
    dir_size(root)
}

/// Recursive directory size via a bounded worker pool.
///
/// Iterative on purpose: arbitrarily deep trees cannot overflow the stack,
/// and the fixed thread count keeps the scan from starving the rest of the
/// app. Unreadable entries are skipped, matching `du`-style tools.
/// Shared work state: the pending directory queue plus a count of directories
/// that are queued OR currently being scanned. When `pending` reaches zero the
/// whole tree is accounted for and every worker may exit.
struct Work {
    queue: Vec<PathBuf>,
    pending: usize,
}

fn dir_size(root: &Path) -> DirSizeSummary {
    let bytes = AtomicU64::new(0);
    let files = AtomicU64::new(0);
    let dirs = AtomicU64::new(0);
    let work = Mutex::new(Work {
        queue: vec![root.to_path_buf()],
        pending: 1,
    });
    let available = Condvar::new();

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                // Take the next directory, or block until one appears / the
                // scan finishes. A condvar replaces a busy-wait so idle workers
                // don't spin the CPU while a few deep branches are scanned.
                let dir = {
                    let mut guard = work.lock();
                    loop {
                        if let Some(dir) = guard.queue.pop() {
                            break dir;
                        }
                        if guard.pending == 0 {
                            // Nothing queued and nothing in flight: done. Wake
                            // any parked siblings so they observe this and exit.
                            available.notify_all();
                            return;
                        }
                        // Re-checks the predicate on wake, so spurious wakeups
                        // and lost notifications are both handled.
                        available.wait(&mut guard);
                    }
                };

                scan_one(&dir, &bytes, &files, &dirs, &work, &available);

                // This directory is now fully accounted for.
                let mut guard = work.lock();
                guard.pending -= 1;
                if guard.pending == 0 {
                    available.notify_all();
                }
            });
        }
    });

    DirSizeSummary {
        bytes: bytes.load(Ordering::SeqCst),
        files: files.load(Ordering::SeqCst),
        dirs: dirs.load(Ordering::SeqCst),
    }
}

fn scan_one(
    dir: &Path,
    bytes: &AtomicU64,
    files: &AtomicU64,
    dirs: &AtomicU64,
    work: &Mutex<Work>,
    available: &Condvar,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        // Symlinks & junctions are skipped entirely: prevents infinite loops and cross-device hopping.
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            dirs.fetch_add(1, Ordering::Relaxed);
            let mut guard = work.lock();
            guard.queue.push(entry.path());
            guard.pending += 1;
            available.notify_one();
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.fetch_add(1, Ordering::Relaxed);
            bytes.fetch_add(size, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_directory, path_size};
    use std::fs;

    #[test]
    fn file_size_is_its_length() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.bin");
        fs::write(&f, vec![0u8; 512]).unwrap();
        let s = path_size(&f);
        assert_eq!((s.bytes, s.files, s.dirs), (512, 1, 0));
    }

    #[test]
    fn dir_size_sums_nested_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a"), vec![0u8; 100]).unwrap();
        fs::create_dir_all(tmp.path().join("x/y")).unwrap();
        fs::write(tmp.path().join("x/b"), vec![0u8; 30]).unwrap();
        fs::write(tmp.path().join("x/y/c"), vec![0u8; 20]).unwrap();

        let s = path_size(tmp.path());
        assert_eq!(s.bytes, 150);
        assert_eq!(s.files, 3);
        assert_eq!(s.dirs, 2);
    }

    #[test]
    fn deep_tree_does_not_overflow_stack() {
        let tmp = tempfile::tempdir().unwrap();
        let mut p = tmp.path().to_path_buf();
        // Deep but under macOS PATH_MAX (1024); the scan itself is iterative,
        // so depth is bounded by path length, not stack.
        for i in 0..200 {
            p.push(format!("d{i}"));
        }
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("leaf"), vec![0u8; 7]).unwrap();

        let s = path_size(tmp.path());
        assert_eq!(s.bytes, 7);
        assert_eq!(s.dirs, 200);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycle_terminates() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("f"), vec![0u8; 10]).unwrap();
        std::os::unix::fs::symlink(tmp.path(), sub.join("loop")).unwrap();

        let s = path_size(tmp.path());
        assert_eq!(s.bytes, 10, "symlink must not be followed");
    }

    #[test]
    fn missing_path_is_zero() {
        assert_eq!(path_size(std::path::Path::new("/no/such/path")).bytes, 0);
    }

    #[test]
    fn wide_tree_drains_across_workers() {
        // Many sibling directories, each with a nested subdir and two files,
        // exercise the condvar queue drain across all worker threads.
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..60 {
            let d = tmp.path().join(format!("d{i}"));
            fs::create_dir_all(d.join("inner")).unwrap();
            fs::write(d.join("a"), vec![0u8; 3]).unwrap();
            fs::write(d.join("inner").join("b"), vec![0u8; 5]).unwrap();
        }
        let s = path_size(tmp.path());
        assert_eq!(s.bytes, 60 * 8);
        assert_eq!(s.files, 120);
        assert_eq!(s.dirs, 120); // 60 top-level + 60 nested
    }

    #[test]
    fn test_analyze_directory() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("big_file.bin"), vec![0u8; 1000]).unwrap();
        let sub = tmp.path().join("sub_folder");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.bin"), vec![0u8; 500]).unwrap();

        let analysis = analyze_directory(tmp.path()).unwrap();
        assert!(analysis.ok);
        assert_eq!(analysis.total_size, 1500);
        assert_eq!(analysis.total_files, 2);
        assert_eq!(analysis.items.len(), 2);
        assert_eq!(analysis.items[0].name, "big_file.bin");
        assert_eq!(analysis.items[0].size, 1000);
        assert_eq!(analysis.items[1].name, "sub_folder");
        assert_eq!(analysis.items[1].size, 500);
    }
}
