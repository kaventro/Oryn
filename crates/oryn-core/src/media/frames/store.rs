use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};

use super::{non_empty, system_source, FrameRequest, FrameSource, VideoFrames};

pub struct FrameStore {
    source: Box<dyn FrameSource>,
    dir: PathBuf,
}

impl FrameStore {
    pub fn new(source: Box<dyn FrameSource>, dir: PathBuf) -> Self {
        Self { source, dir }
    }

    pub fn system() -> Self {
        Self::new(system_source(), std::env::temp_dir().join("oryn-frames"))
    }

    fn key(video: &Path, size: u32) -> Option<String> {
        let meta = std::fs::metadata(video).ok()?;
        let stamp = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let mut h = Sha256::new();
        h.update(video.as_os_str().as_encoded_bytes());
        h.update(meta.len().to_le_bytes());
        h.update(stamp.to_le_bytes());
        h.update(size.to_le_bytes());
        Some(hex::encode(&h.finalize()[..16]))
    }
}

impl VideoFrames for FrameStore {
    fn frame(&self, video: &Path, size: u32) -> Option<PathBuf> {
        let video = std::fs::canonicalize(video).ok()?;
        if !video.is_file() {
            return None;
        }
        let out = self.dir.join(format!("{}.png", Self::key(&video, size)?));
        if non_empty(&out) {
            return Some(out);
        }
        private_dir(&self.dir)?;
        let request = FrameRequest { video, out, size };
        self.source.render(&request).filter(|p| non_empty(p))
    }
}

#[cfg(unix)]
fn private_dir(path: &Path) -> Option<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .ok()?;
    let meta = std::fs::symlink_metadata(path).ok()?;
    (meta.is_dir() && meta.permissions().mode() & 0o077 == 0).then_some(())
}

#[cfg(not(unix))]
fn private_dir(path: &Path) -> Option<()> {
    std::fs::create_dir_all(path).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    struct Writes(Arc<AtomicUsize>);

    impl FrameSource for Writes {
        fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
            self.0.fetch_add(1, Ordering::SeqCst);
            assert!(request.video.is_absolute());
            std::fs::write(&request.out, b"png").ok()?;
            Some(request.out.clone())
        }
    }

    #[test]
    fn renders_once_and_then_reuses_the_frame() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("a.mp4");
        std::fs::write(&video, b"video").unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let store = FrameStore::new(Box::new(Writes(calls.clone())), dir.path().join("frames"));
        let first = store.frame(&video, 512).unwrap();
        let again = store.frame(&video, 512).unwrap();
        assert_eq!(first, again);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_ne!(store.frame(&video, 256).unwrap(), first);
        std::fs::write(&video, b"a longer video").unwrap();
        assert_ne!(store.frame(&video, 512).unwrap(), first);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn ignores_missing_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let store = FrameStore::new(Box::new(Writes(calls.clone())), dir.path().join("frames"));
        assert!(store.frame(&dir.path().join("gone.mp4"), 64).is_none());
        assert!(store.frame(dir.path(), 64).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(unix)]
    #[test]
    fn keeps_frames_in_a_private_folder() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let frames = dir.path().join("frames");
        private_dir(&frames).unwrap();
        let mode = std::fs::metadata(&frames).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        std::fs::set_permissions(&frames, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(private_dir(&frames).is_none());
    }
}
