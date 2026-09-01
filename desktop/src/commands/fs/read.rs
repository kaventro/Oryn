use serde_json::Value;
use std::fs;

use crate::services::fs_props::StatPropsOut;
use crate::services::fs_vfs::DirListing;
use crate::services::{fs_config, fs_props, fs_size, fs_vfs};

pub fn config_load() -> Result<Value, String> {
    fs_config::load_config().map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDirIn {
    pub path: String,
}

pub fn fs_read_dir(input: ReadDirIn) -> Result<DirListing, String> {
    fs_vfs::read_dir_response(&input.path).map_err(|e| e.to_string())
}

pub async fn fs_read_flat_branch(input: ReadDirIn) -> Result<DirListing, String> {
    let path = std::path::PathBuf::from(&input.path);
    tokio::task::spawn_blocking(move || {
        let items = crate::services::fs_listing::list_flat_branch(&path, 50_000)?;
        let file_items: Vec<crate::vfs::FileItem> = items
            .into_iter()
            .map(|it| crate::vfs::FileItem {
                display: it.name.clone(),
                base: it.name,
                is_dir: false,
                size: it.size,
                mtime: it.mtime,
            })
            .collect();
        Ok(DirListing {
            ok: true,
            items: file_items,
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: anyhow::Error| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatIn {
    pub path: String,
}

pub fn fs_stat_props(input: StatIn) -> Result<StatPropsOut, String> {
    fs_props::stat_props_response(&input.path).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileIn {
    pub path: String,
    pub max_bytes: Option<usize>,
}

const MAX_ALLOWED_FILE_READ_BYTES: usize = 10 * 1024 * 1024; // 10 MB maximum hard ceiling

pub fn fs_read_file_text(input: ReadFileIn) -> Result<String, String> {
    use std::io::Read;
    let max = input
        .max_bytes
        .unwrap_or(512_000)
        .min(MAX_ALLOWED_FILE_READ_BYTES);
    let file = fs::File::open(&input.path).map_err(|e| e.to_string())?;
    let mut reader = file.take(max as u64);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Reports whether the leading bytes look like text, so the viewer can refuse a
/// binary rather than rendering `from_utf8_lossy`'s replacement characters. A
/// NUL byte in the first block is the same signal `git` and `grep` use.
pub fn fs_probe_text(input: ReadFileIn) -> Result<TextProbe, String> {
    use std::io::Read;
    let file = fs::File::open(&input.path).map_err(|e| e.to_string())?;
    let mut head = Vec::new();
    file.take(8192)
        .read_to_end(&mut head)
        .map_err(|e| e.to_string())?;

    let control = head
        .iter()
        .filter(|&&b| b < 0x09 || (0x0d < b && b < 0x20))
        .count();
    let is_text = !head.contains(&0) && control * 100 <= head.len().max(1) * 5;

    Ok(TextProbe { is_text })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProbe {
    pub is_text: bool,
}

pub fn fs_read_office(input: ReadFileIn) -> Result<crate::services::fs_office::OfficeDoc, String> {
    crate::services::fs_office::read_office(&input.path).map_err(|e| e.to_string())
}

pub fn fs_read_media_data_url(input: ReadFileIn) -> Result<String, String> {
    use std::io::Read;
    let path = std::path::Path::new(&input.path);
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" | "aac" => "audio/aac",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    };

    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    const MAX_ALLOWED_MEDIA_BYTES: usize = 20_000_000; // 20 MB ceiling
    let max = input
        .max_bytes
        .unwrap_or(MAX_ALLOWED_MEDIA_BYTES)
        .min(MAX_ALLOWED_MEDIA_BYTES);
    let mut reader = f.take(max as u64);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirSizeIn {
    pub path: String,
}

#[derive(serde::Serialize)]
pub struct DirSizeOut {
    pub ok: bool,
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
}

pub async fn fs_get_dir_size(input: DirSizeIn) -> Result<DirSizeOut, String> {
    let path = std::path::PathBuf::from(&input.path);
    let summary = tokio::task::spawn_blocking(move || fs_size::path_size(&path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(DirSizeOut {
        ok: true,
        size: summary.bytes,
        files: summary.files,
        dirs: summary.dirs,
    })
}

pub async fn fs_analyze_dir(input: DirSizeIn) -> Result<fs_size::DiskSpaceAnalysis, String> {
    let path = std::path::PathBuf::from(&input.path);
    tokio::task::spawn_blocking(move || fs_size::analyze_directory(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesIn {
    pub path: String,
    pub min_size_bytes: Option<u64>,
    pub max_results: Option<usize>,
}

pub async fn fs_scan_duplicates(
    input: DuplicatesIn,
) -> Result<crate::services::fs_duplicates::DuplicateScanResult, String> {
    let opts = crate::services::fs_duplicates::DuplicateScanOptions {
        path: input.path,
        min_size_bytes: input.min_size_bytes,
        max_results: input.max_results,
    };
    tokio::task::spawn_blocking(move || crate::services::fs_duplicates::scan_duplicates(opts))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumIn {
    pub path: String,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumOut {
    pub ok: bool,
    pub path: String,
    pub file_name: String,
    pub size: u64,
    pub md5: String,
    pub sha256: String,
}

pub async fn fs_checksum(input: ChecksumIn) -> Result<ChecksumOut, String> {
    let path_str = input.path;
    let path = std::path::PathBuf::from(&path_str);
    if !path.exists() {
        return Err(format!("File not found: {}", path_str));
    }
    if path.is_dir() {
        return Err(format!("Path is a directory, not a file: {}", path_str));
    }

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path_str.clone());

    let path_clone = path.clone();
    let res = tokio::task::spawn_blocking(move || -> std::io::Result<(u64, String, String)> {
        use md5::Md5;
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = std::fs::File::open(&path_clone)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut sha256_hasher = Sha256::new();
        let mut md5_hasher = Md5::new();

        let mut buffer = [0u8; 65536];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            sha256_hasher.update(&buffer[..bytes_read]);
            md5_hasher.update(&buffer[..bytes_read]);
        }

        let sha256_res = hex::encode(sha256_hasher.finalize());
        let md5_res = hex::encode(md5_hasher.finalize());

        Ok((size, md5_res, sha256_res))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(ChecksumOut {
        ok: true,
        path: path_str,
        file_name,
        size: res.0,
        md5: res.1,
        sha256: res.2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn computes_correct_checksums() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "hello world").unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let res = fs_checksum(ChecksumIn { path }).await.unwrap();

        assert!(res.ok);
        assert_eq!(res.size, 12);
        // MD5 of "hello world\n" is 6f5902ac237024bdd0c176cb93063dc4
        assert_eq!(res.md5, "6f5902ac237024bdd0c176cb93063dc4");
        // SHA256 of "hello world\n" is a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447
        assert_eq!(
            res.sha256,
            "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447"
        );
    }

    #[test]
    fn reads_media_as_data_url() {
        let mut tmp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        tmp.write_all(b"\x89PNG\r\n\x1a\nfakeimage").unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let data_url = fs_read_media_data_url(ReadFileIn {
            path,
            max_bytes: None,
        })
        .unwrap();

        assert!(data_url.starts_with("data:image/png;base64,"));
    }
}
