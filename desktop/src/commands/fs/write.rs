use std::fs;

use crate::commands::response::{ack, Ack};
use crate::services::{fs_config, fs_delete, fs_rename, fs_zip};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameIn {
    pub src: String,
    pub dst: String,
}

pub fn fs_rename(input: RenameIn) -> Result<Ack, String> {
    fs_rename::rename(&input.src, &input.dst).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteIn {
    pub full_path: String,
    pub use_trash: Option<bool>,
}

pub fn fs_delete(input: DeleteIn) -> Result<Ack, String> {
    let options = fs_config::resolve_delete_options(input.use_trash).map_err(|e| e.to_string())?;
    fs_delete::delete_path(
        &input.full_path,
        options.use_trash,
        options.log_path.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(ack())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipIn {
    pub path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileIn {
    pub path: String,
    pub content: Option<String>,
}

fn validate_safe_write_path(path: &std::path::Path) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err("Writing to symlink destination is prohibited for security".to_string());
        }
    }
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent.as_os_str().is_empty() || parent == std::path::Path::new("/") {
            break;
        }
        #[cfg(target_os = "macos")]
        if parent == std::path::Path::new("/var")
            || parent == std::path::Path::new("/tmp")
            || parent == std::path::Path::new("/etc")
        {
            break;
        }
        if let Ok(meta) = fs::symlink_metadata(parent) {
            if meta.file_type().is_symlink() {
                return Err(
                    "Writing through symlink directory components is prohibited for security"
                        .to_string(),
                );
            }
        }
        current = parent.parent();
    }
    Ok(())
}

fn safe_write_file(path: &std::path::Path, content: &[u8], create_new: bool) -> Result<(), String> {
    if create_new && path.exists() {
        return Err("File already exists".to_string());
    }
    validate_safe_write_path(path)?;

    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    validate_safe_write_path(parent)?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(".tmp-write-{}-{}", std::process::id(), nonce));

    if let Ok(meta) = fs::symlink_metadata(&tmp_path) {
        if meta.file_type().is_symlink() {
            let _ = fs::remove_file(&tmp_path);
        }
    }

    let write_res = (|| -> Result<(), std::io::Error> {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.to_string());
    }

    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            let _ = fs::remove_file(&tmp_path);
            return Err("Writing to symlink destination is prohibited for security".to_string());
        }
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.to_string());
    }

    Ok(())
}

pub fn fs_create_file(input: CreateFileIn) -> Result<Ack, String> {
    let path = std::path::Path::new(&input.path);
    let content = input.content.as_deref().unwrap_or("").as_bytes();
    safe_write_file(path, content, true)?;
    Ok(ack())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteFileTextIn {
    pub path: String,
    pub content: String,
}

pub fn fs_write_file_text(input: WriteFileTextIn) -> Result<Ack, String> {
    let path = std::path::Path::new(&input.path);
    safe_write_file(path, input.content.as_bytes(), false)?;
    Ok(ack())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MkdirIn {
    pub path: String,
}

pub fn fs_mkdir(input: MkdirIn) -> Result<Ack, String> {
    fs::create_dir_all(&input.path).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipCreated {
    pub ok: bool,
    pub zip_path: String,
}

pub fn fs_compress_zip(input: ZipIn) -> Result<ZipCreated, String> {
    let zip_path = fs_zip::compress_zip(&input.path).map_err(|e| e.to_string())?;
    Ok(ZipCreated { ok: true, zip_path })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressIn {
    pub sources: Vec<String>,
    pub destination: String,
}

pub fn fs_compress(input: CompressIn) -> Result<ZipCreated, String> {
    let src_paths: Vec<std::path::PathBuf> =
        input.sources.iter().map(std::path::PathBuf::from).collect();
    let dst_path = std::path::PathBuf::from(&input.destination);
    crate::services::fs_archive::compress_zip(&src_paths, &dst_path).map_err(|e| e.to_string())?;
    Ok(ZipCreated {
        ok: true,
        zip_path: input.destination,
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractIn {
    pub archive: String,
    pub destination: String,
}

pub fn fs_extract(input: ExtractIn) -> Result<Ack, String> {
    let arch_path = std::path::PathBuf::from(&input.archive);
    let dst_path = std::path::PathBuf::from(&input.destination);
    crate::services::fs_archive::extract_archive(&arch_path, &dst_path)
        .map_err(|e| e.to_string())?;
    Ok(ack())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_safe_write_file_creates_and_overwrites() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("sub").join("test.txt");

        safe_write_file(&target, b"hello", true).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");

        // create_new = true on existing file should fail
        assert!(safe_write_file(&target, b"world", true).is_err());

        // create_new = false should succeed and overwrite atomically
        safe_write_file(&target, b"world", false).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "world");
    }
}
