use std::path::{Path, PathBuf};

#[cfg(test)]
pub static OPENED: parking_lot::Mutex<Vec<PathBuf>> = parking_lot::Mutex::new(Vec::new());

pub fn open(path: &Path) {
    #[cfg(test)]
    OPENED.lock().push(path.to_path_buf());
    #[cfg(not(test))]
    let _ = open::that_detached(path);
}

pub fn temp_copy(name: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join("oryn-archive");
    private_dir(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let sub = dir.join(stamp.to_string());
    private_dir(&sub)?;
    Some(sub.join(name))
}

#[cfg(unix)]
fn private_dir(path: &Path) -> Option<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .ok()
}

#[cfg(not(unix))]
fn private_dir(path: &Path) -> Option<()> {
    std::fs::create_dir_all(path).ok()
}
