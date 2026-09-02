use std::fs;
use std::io::Write;
use std::path::Path;

use crate::services::ServiceResult;

pub fn delete_path(full_path: &str, use_trash: bool, log_path: Option<&Path>) -> ServiceResult<()> {
    let mut cleaned = full_path.trim();
    let is_root = cleaned == "/"
        || cleaned.eq_ignore_ascii_case("\\")
        || (cleaned.len() == 2 && cleaned.as_bytes()[1] == b':')
        || (cleaned.len() == 3
            && cleaned.as_bytes()[1] == b':'
            && (cleaned.ends_with('\\') || cleaned.ends_with('/')));
    if !is_root {
        cleaned = cleaned.trim_end_matches(['/', '\\']);
    }
    if cleaned.is_empty() {
        anyhow::bail!("Refusing to delete an empty path");
    }

    let path = Path::new(cleaned);
    if use_trash {
        // AppKit trash from a Tauri worker thread can report success without
        // moving the file. Always fall through to unlink if it is still there.
        let _ = trash::delete(cleaned);
    }
    if path.symlink_metadata().is_ok() {
        unlink(path)?;
    }

    if let Some(log_path) = log_path {
        let line = format!("{}\t{}\n", chrono::Utc::now().to_rfc3339(), full_path);
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .and_then(|mut file| file.write_all(line.as_bytes()))
        {
            tracing::warn!("failed to write deletion audit log: {e}");
        }
    }

    Ok(())
}

fn unlink(path: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn deletes_file_without_trash() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("gone.txt");
        fs::write(&file, "x").unwrap();
        delete_path(file.to_str().unwrap(), false, None).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn deletes_file_even_when_trash_is_requested() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("gone-trash.txt");
        fs::write(&file, "x").unwrap();
        delete_path(file.to_str().unwrap(), true, None).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn deletes_directory_without_trash() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("folder");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("a.txt"), "x").unwrap();
        let with_slash = format!("{}/", nested.to_str().unwrap());
        delete_path(&with_slash, false, None).unwrap();
        assert!(!nested.exists());
    }

    #[test]
    fn refuses_empty_path() {
        assert!(delete_path("   ", false, None).is_err());
    }
}
