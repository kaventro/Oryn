use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::services::fs_transfer::sink::ProgressSink;
use crate::vfs::{VfsRouter, VirtualFileSystem};

/// What to do when the destination already exists.
/// Resolved by the UI (one dialog per operation) before the engine runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverwritePolicy {
    #[default]
    Overwrite,
    Skip,
}

impl OverwritePolicy {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("skip") => OverwritePolicy::Skip,
            _ => OverwritePolicy::Overwrite,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CopyStats {
    pub copied: u64,
    pub skipped_existing: u64,
    pub symlinks: u64,
    pub symlinks_skipped: u64,
}

impl CopyStats {
    /// True when every source item made it to the destination, so a move
    /// may safely delete the source tree.
    pub fn is_complete(&self) -> bool {
        self.skipped_existing == 0 && self.symlinks_skipped == 0
    }
}

#[derive(Debug)]
pub enum CopyError {
    Aborted,
    Failed(String),
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopyError::Aborted => write!(f, "aborted"),
            CopyError::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for CopyError {
    fn from(e: std::io::Error) -> Self {
        CopyError::Failed(e.to_string())
    }
}

/// Recursive copier: walks the source tree, honours the overwrite policy,
/// recreates symlinks instead of silently dropping them, and preserves
/// permissions + mtime on everything it writes.
pub struct CopyEngine<'a> {
    sink: &'a dyn ProgressSink,
    policy: OverwritePolicy,
    rel_base: PathBuf,
    stats: CopyStats,
}

impl<'a> CopyEngine<'a> {
    pub fn new(sink: &'a dyn ProgressSink, policy: OverwritePolicy, rel_base: &Path) -> Self {
        Self {
            sink,
            policy,
            rel_base: rel_base.to_path_buf(),
            stats: CopyStats::default(),
        }
    }

    pub fn run(mut self, src: &Path, dst: &Path) -> Result<CopyStats, CopyError> {
        Self::guard_dst_not_inside_src(src, dst)?;
        self.copy_tree(src, dst)?;
        Ok(self.stats)
    }

    /// Reject copying/moving a real directory into itself or one of its own
    /// descendants. Without this, `copy_tree` re-reads the directory it is
    /// writing into and recurses until the disk fills or the stack overflows
    /// (which aborts the whole app under `panic = "abort"`).
    fn guard_dst_not_inside_src(src: &Path, dst: &Path) -> Result<(), CopyError> {
        let is_real_dir = fs::symlink_metadata(src)
            .map(|m| m.file_type().is_dir())
            .unwrap_or(false);
        if !is_real_dir {
            return Ok(());
        }
        let Ok(src_can) = fs::canonicalize(src) else {
            return Ok(());
        };
        // `dst` may not exist yet; resolve its nearest existing ancestor.
        let mut probe = dst.to_path_buf();
        let dst_can = loop {
            if let Ok(resolved) = fs::canonicalize(&probe) {
                break resolved;
            }
            if !probe.pop() {
                return Ok(());
            }
        };
        // `starts_with` is component-wise and matches equality too, so this
        // covers both "into itself" and "into a subfolder of itself".
        if dst_can.starts_with(&src_can) {
            return Err(CopyError::Failed(
                "Cannot copy or move a folder into itself or one of its own subfolders.".into(),
            ));
        }
        Ok(())
    }

    fn check_abort(&self) -> Result<(), CopyError> {
        if self.sink.is_aborted() {
            return Err(CopyError::Aborted);
        }
        Ok(())
    }

    fn rel_of(&self, path: &Path) -> String {
        path.strip_prefix(&self.rel_base)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    fn name_of(path: &Path) -> String {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    fn copy_tree(&mut self, src: &Path, dst: &Path) -> Result<(), CopyError> {
        self.check_abort()?;

        if is_archive_inner_path(src) {
            return self.extract_from_archive(src, dst);
        }

        let meta = fs::symlink_metadata(src)?;
        let ft = meta.file_type();

        if ft.is_symlink() {
            return self.copy_symlink(src, dst);
        }
        if ft.is_file() {
            return self.copy_file_checked(src, dst, &meta);
        }
        if !ft.is_dir() {
            // Sockets, fifos, devices: nothing sensible to copy.
            return Ok(());
        }

        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            self.copy_tree(&src.join(entry.file_name()), &dst.join(entry.file_name()))?;
        }
        preserve_metadata(&meta, dst);
        Ok(())
    }

    fn extract_from_archive(&mut self, src: &Path, dst: &Path) -> Result<(), CopyError> {
        VfsRouter::new()
            .extract_to(&src.to_string_lossy(), dst)
            .map_err(|e| CopyError::Failed(e.to_string()))?;
        self.stats.copied += 1;
        self.sink
            .emit(json!({"type": "done", "path": Self::name_of(src)}));
        Ok(())
    }

    fn copy_symlink(&mut self, src: &Path, dst: &Path) -> Result<(), CopyError> {
        let target = fs::read_link(src)?;
        if fs::symlink_metadata(dst).is_ok() {
            if self.policy == OverwritePolicy::Skip {
                self.stats.skipped_existing += 1;
                self.sink
                    .emit(json!({"type": "skip", "path": self.rel_of(src)}));
                return Ok(());
            }
            remove_existing(dst)?;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, dst)?;
            self.stats.symlinks += 1;
            self.sink
                .emit(json!({"type": "done", "path": Self::name_of(src)}));
        }
        #[cfg(not(unix))]
        {
            // Creating symlinks on Windows needs elevated rights; report
            // instead of failing the whole transfer.
            let _ = target;
            self.stats.symlinks_skipped += 1;
            self.sink
                .emit(json!({"type": "skip", "path": self.rel_of(src), "reason": "symlink"}));
        }
        Ok(())
    }

    fn copy_file_checked(
        &mut self,
        src: &Path,
        dst: &Path,
        meta: &fs::Metadata,
    ) -> Result<(), CopyError> {
        if fs::symlink_metadata(dst).is_ok() && self.policy == OverwritePolicy::Skip {
            self.stats.skipped_existing += 1;
            self.sink
                .emit(json!({"type": "skip", "path": self.rel_of(src)}));
            return Ok(());
        }

        self.copy_file_with_progress(src, dst)?;
        preserve_metadata(meta, dst);
        self.stats.copied += 1;
        self.sink
            .emit(json!({"type": "done", "path": Self::name_of(src)}));
        Ok(())
    }

    fn copy_file_with_progress(&self, src: &Path, dst: &Path) -> Result<(), CopyError> {
        self.check_abort()?;

        let parent = dst
            .parent()
            .ok_or_else(|| CopyError::Failed("no parent".to_string()))?;
        fs::create_dir_all(parent)?;
        let name = dst
            .file_name()
            .ok_or_else(|| CopyError::Failed("no file name".to_string()))?;

        let total = fs::metadata(src)?.len();
        let mut reader = fs::File::open(src)?;
        let root =
            crate::fs_safe::SafeRoot::open(parent).map_err(|e| CopyError::Failed(e.to_string()))?;
        let mut writer = root
            .create_file(Path::new(name), None)
            .map_err(|e| CopyError::Failed(e.to_string()))?;
        let mut buf = vec![0u8; 256 * 1024];
        let mut done = 0u64;
        let name = Self::name_of(src);
        let rel = self.rel_of(src);

        loop {
            self.check_abort()?;
            let read = reader.read(&mut buf)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buf[..read])?;
            done += read as u64;

            self.sink.emit(json!({
                "type": "file",
                "path": name,
                "bytes": done,
                "total": total,
                "file": src.to_string_lossy().to_string(),
                "rel": rel,
            }));
        }

        Ok(())
    }
}

/// Best effort: failing to copy permissions or mtime must not fail the
/// transfer itself (e.g. FAT volumes without permission bits).
fn preserve_metadata(src_meta: &fs::Metadata, dst: &Path) {
    let _ = fs::set_permissions(dst, src_meta.permissions());
    let mtime = filetime::FileTime::from_last_modification_time(src_meta);
    let _ = filetime::set_file_mtime(dst, mtime);
}

fn remove_existing(dst: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(dst) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(dst),
        Ok(_) => fs::remove_file(dst),
        Err(_) => Ok(()),
    }
}

/// Source lives inside a zip/tar archive and must go through the VFS.
fn is_archive_inner_path(src: &Path) -> bool {
    let text = src.to_string_lossy().to_lowercase();
    let markers = [
        ".zip/",
        ".tar/",
        ".tar.gz/",
        ".tar.bz2/",
        ".tar.xz/",
        ".tar.zst/",
        ".tgz/",
    ];
    markers.iter().any(|m| text.contains(m))
}

pub(crate) fn should_copy_instead_of_rename(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::EXDEV)
    }

    #[cfg(windows)]
    {
        error.raw_os_error() == Some(17)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct TestSink {
        aborted: AtomicBool,
        events: Mutex<Vec<Value>>,
    }

    impl ProgressSink for TestSink {
        fn emit(&self, payload: Value) {
            self.events.lock().unwrap().push(payload);
        }
        fn is_aborted(&self) -> bool {
            self.aborted.load(Ordering::SeqCst)
        }
    }

    fn run_copy(src: &Path, dst: &Path, policy: OverwritePolicy) -> CopyStats {
        let sink = TestSink::default();
        CopyEngine::new(&sink, policy, src)
            .run(src, dst)
            .expect("copy should succeed")
    }

    #[test]
    fn copies_tree_with_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), b"alpha").unwrap();
        fs::write(src.join("sub/b.txt"), b"beta").unwrap();

        let stats = run_copy(&src, &dst, OverwritePolicy::Overwrite);

        assert_eq!(stats.copied, 2);
        assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(fs::read(dst.join("sub/b.txt")).unwrap(), b"beta");
    }

    #[test]
    fn skip_policy_keeps_existing_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("f.txt"), b"new").unwrap();
        fs::write(dst.join("f.txt"), b"old").unwrap();

        let stats = run_copy(&src, &dst, OverwritePolicy::Skip);

        assert_eq!(stats.skipped_existing, 1);
        assert_eq!(stats.copied, 0);
        assert_eq!(
            fs::read(dst.join("f.txt")).unwrap(),
            b"old",
            "must not clobber"
        );
        assert!(!stats.is_complete());
    }

    #[test]
    fn overwrite_policy_replaces_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("f.txt"), b"new").unwrap();
        fs::write(dst.join("f.txt"), b"old").unwrap();

        let stats = run_copy(&src, &dst, OverwritePolicy::Overwrite);

        assert_eq!(stats.copied, 1);
        assert_eq!(fs::read(dst.join("f.txt")).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_recreated_not_dropped() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("real.txt"), b"data").unwrap();
        std::os::unix::fs::symlink("real.txt", src.join("link.txt")).unwrap();

        let stats = run_copy(&src, &dst, OverwritePolicy::Overwrite);

        assert_eq!(stats.symlinks, 1);
        let target = fs::read_link(dst.join("link.txt")).expect("must be a symlink");
        assert_eq!(target.to_str(), Some("real.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn permissions_and_mtime_survive_the_copy() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        let f = src.join("script.sh");
        fs::write(&f, b"#!/bin/sh\n").unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o755)).unwrap();
        filetime::set_file_mtime(&f, filetime::FileTime::from_unix_time(1_600_000_000, 0)).unwrap();

        run_copy(&src, &dst, OverwritePolicy::Overwrite);

        let out = fs::metadata(dst.join("script.sh")).unwrap();
        assert_eq!(out.permissions().mode() & 0o777, 0o755);
        let mtime = filetime::FileTime::from_last_modification_time(&out);
        assert_eq!(mtime.unix_seconds(), 1_600_000_000);
    }

    #[test]
    fn abort_stops_the_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("f.txt"), b"x").unwrap();

        let sink = TestSink::default();
        sink.aborted.store(true, Ordering::SeqCst);
        let err = CopyEngine::new(&sink, OverwritePolicy::Overwrite, &src)
            .run(&src, &tmp.path().join("dst"))
            .unwrap_err();
        assert!(matches!(err, CopyError::Aborted));
    }
}
