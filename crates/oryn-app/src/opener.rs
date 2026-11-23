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
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let sub = dir.join(stamp.to_string());
    std::fs::create_dir_all(&sub).ok()?;
    Some(sub.join(name))
}
