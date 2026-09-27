use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

const PREVIEW_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif", "mp3", "wav", "ogg", "m4a",
    "aac", "flac", "mp4", "m4v", "webm", "mov",
];

const SENSITIVE_DIRECTORIES: &[&str] = &[
    ".ssh", ".gnupg", ".gpg", ".aws", ".azure", ".kube", ".docker", ".npm", ".cargo", ".git",
    ".svn", ".hg",
];

pub fn validate_preview_asset(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("preview path is empty".into());
    }

    let requested = Path::new(path);
    if has_sensitive_component(requested) {
        return Err("preview path is not allowed".into());
    }

    let canonical = std::fs::canonicalize(requested).map_err(|e| e.to_string())?;
    if has_sensitive_component(&canonical) {
        return Err("preview path is not allowed".into());
    }

    let meta = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("preview path is not a file".into());
    }

    let ext = canonical
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
        return Err("preview path is not a media file".into());
    }

    Ok(canonical)
}

fn has_sensitive_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => is_sensitive_dir(name),
        _ => false,
    })
}

fn is_sensitive_dir(name: &OsStr) -> bool {
    let lower = name.to_string_lossy().to_ascii_lowercase();
    SENSITIVE_DIRECTORIES.iter().any(|dir| lower == *dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn allows_a_regular_image() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("photo.PNG");
        std::fs::write(&file, b"png").unwrap();

        let granted = validate_preview_asset(file.to_str().unwrap()).unwrap();
        assert_eq!(granted.file_name().unwrap(), "photo.PNG");
    }

    #[test]
    fn rejects_non_media_and_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = tmp.path().join("notes.txt");
        let mut f = std::fs::File::create(&notes).unwrap();
        writeln!(f, "secret").unwrap();

        assert!(validate_preview_asset(notes.to_str().unwrap()).is_err());
        assert!(validate_preview_asset(tmp.path().to_str().unwrap()).is_err());
        assert!(validate_preview_asset("").is_err());
        assert!(validate_preview_asset("   ").is_err());
    }

    #[test]
    fn rejects_sensitive_directories_even_for_images() {
        let tmp = tempfile::tempdir().unwrap();
        let ssh = tmp.path().join(".ssh");
        std::fs::create_dir(&ssh).unwrap();
        let key = ssh.join("id_rsa.png");
        std::fs::write(&key, b"not-a-key").unwrap();

        assert!(validate_preview_asset(key.to_str().unwrap()).is_err());

        let via_parent = tmp.path().join("other").join("..").join(".ssh").join("id_rsa.png");
        assert!(validate_preview_asset(via_parent.to_str().unwrap()).is_err());
    }

    #[test]
    fn rejects_a_file_without_a_preview_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("noext");
        std::fs::write(&file, b"bytes").unwrap();
        assert!(validate_preview_asset(file.to_str().unwrap()).is_err());
    }

    #[test]
    fn rejects_a_link_whose_canonical_path_is_sensitive() {
        let tmp = tempfile::tempdir().unwrap();
        let ssh = tmp.path().join(".ssh");
        std::fs::create_dir(&ssh).unwrap();
        let secret = ssh.join("key.png");
        std::fs::write(&secret, b"png").unwrap();

        let alias = tmp.path().join("photo.png");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &alias).unwrap();
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_file(&secret, &alias).is_err() {
                return;
            }
        }

        let err = validate_preview_asset(alias.to_str().unwrap()).unwrap_err();
        assert!(err.contains("not allowed"));
    }
}
