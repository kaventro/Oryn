use anyhow::{bail, Context};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::services::ServiceResult;

/// Normalizes a path for comparison across platforms.
/// Handles mixed slashes, casing (on Windows/macOS), and trailing slashes.
fn normalize_parent(p: &Path) -> PathBuf {
    if let Ok(canon) = p.canonicalize() {
        return canon;
    }
    let mut normalized = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            c => normalized.push(c.as_os_str()),
        }
    }
    normalized
}

pub fn rename(src: &str, dst: &str) -> ServiceResult<()> {
    let src_clean = src.trim_end_matches(['/', '\\']);
    let dst_clean = dst.trim_end_matches(['/', '\\']);
    let src_path = Path::new(src_clean);
    let dst_path = Path::new(dst_clean);

    // Ensure src exists
    if !src_path.exists() {
        bail!("Source file does not exist: {src}");
    }

    // Parent directories must match (rename cannot move outside folder)
    let src_parent = src_path.parent().unwrap_or(Path::new(""));
    let dst_parent = dst_path.parent().unwrap_or(Path::new(""));

    let src_norm = normalize_parent(src_parent);
    let dst_norm = normalize_parent(dst_parent);

    #[cfg(windows)]
    let parents_match = {
        let s1 = src_norm.to_string_lossy().to_lowercase();
        let s2 = dst_norm.to_string_lossy().to_lowercase();
        s1 == s2 || src_parent == dst_parent
    };

    #[cfg(not(windows))]
    let parents_match = src_norm == dst_norm || src_parent == dst_parent;

    if !parents_match {
        bail!("Rename destination cannot change parent directory (use move instead)");
    }

    // Validate new basename
    let dst_name = match dst_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.trim(),
        None => bail!("Invalid destination file name"),
    };

    if dst_name.is_empty() || dst_name == "." || dst_name == ".." {
        bail!("Invalid file name: {dst_name:?}");
    }

    if dst_name.contains('/') || dst_name.contains('\\') || dst_name.contains('\0') {
        bail!("File name cannot contain path separators or null bytes");
    }

    // Windows filename constraints (cross-platform enforcement for safety)
    const FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if dst_name
        .chars()
        .any(|c| FORBIDDEN_CHARS.contains(&c) || (c as u32) < 32)
    {
        bail!("File name contains invalid characters: {dst_name:?}");
    }

    // Windows reserved device names
    let stem = dst_name
        .split('.')
        .next()
        .unwrap_or(dst_name)
        .to_ascii_uppercase();
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED_NAMES.contains(&stem.as_str()) {
        bail!("File name is a reserved device name on Windows: {dst_name:?}");
    }

    if dst_name.ends_with('.') || dst_name.ends_with(' ') {
        bail!("File name cannot end with a period or space on Windows");
    }

    fs::rename(src_path, dst_path).with_context(|| format!("Failed to rename {src} to {dst}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validates_rename_basename_only() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("foo.txt");
        fs::write(&file, "hello").unwrap();

        // Valid rename
        let valid_dst = dir.path().join("bar.txt");
        assert!(rename(file.to_str().unwrap(), valid_dst.to_str().unwrap()).is_ok());
        assert!(!file.exists());
        assert!(valid_dst.exists());

        // Path traversal rejection
        let invalid_dst = dir.path().join("../escaped.txt");
        assert!(rename(valid_dst.to_str().unwrap(), invalid_dst.to_str().unwrap()).is_err());
    }

    #[test]
    fn renames_directory_with_trailing_slash() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("my_folder");
        fs::create_dir(&sub).unwrap();

        let src_with_slash = format!("{}/", sub.to_str().unwrap());
        let dst = dir.path().join("renamed_folder");
        let dst_with_slash = format!("{}/", dst.to_str().unwrap());

        assert!(rename(&src_with_slash, &dst_with_slash).is_ok());
        assert!(!sub.exists());
        assert!(dst.exists());
    }

    #[test]
    fn rejects_windows_reserved_names_and_invalid_chars() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let con_dst = dir.path().join("CON.txt");
        assert!(rename(file.to_str().unwrap(), con_dst.to_str().unwrap()).is_err());

        let aux_dst = dir.path().join("aux");
        assert!(rename(file.to_str().unwrap(), aux_dst.to_str().unwrap()).is_err());

        let invalid_char_dst = dir.path().join("bad:name.txt");
        assert!(rename(file.to_str().unwrap(), invalid_char_dst.to_str().unwrap()).is_err());

        let trailing_dot = dir.path().join("name.");
        assert!(rename(file.to_str().unwrap(), trailing_dot.to_str().unwrap()).is_err());
    }
}
