use std::fs;
use std::io::Write;
use std::path::Path;

use crate::services::ServiceResult;

pub fn delete_path(full_path: &str, use_trash: bool, log_path: Option<&Path>) -> ServiceResult<()> {
    if use_trash {
        trash::delete(full_path)?;
    } else {
        let path = Path::new(full_path);
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
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
