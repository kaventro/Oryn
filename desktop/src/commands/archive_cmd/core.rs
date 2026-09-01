use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive as TarArchive;
use tar::Builder as TarBuilder;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArchiveIn {
    pub sources: Vec<String>,
    pub destination: String,
    pub format: Option<String>,
    pub password: Option<String>,
    pub compression_level: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractArchiveIn {
    pub archive_path: String,
    pub destination_dir: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArchiveIn {
    pub archive_path: String,
    #[allow(dead_code)]
    pub password: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryInfo {
    pub name: String,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
    pub is_encrypted: bool,
}

pub fn create_archive(input: CreateArchiveIn) -> Result<Value, String> {
    create_archive_impl(input).map_err(|e| e.to_string())
}

fn create_archive_impl(input: CreateArchiveIn) -> Result<Value> {
    if input.sources.is_empty() {
        return Err(anyhow!("No source files provided for archive"));
    }

    let dest_path = PathBuf::from(&input.destination);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {:?}", dest_path))?;
    }

    let fmt = input
        .format
        .as_deref()
        .unwrap_or_else(|| {
            let lower = input.destination.to_lowercase();
            if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
                "tar.gz"
            } else {
                "zip"
            }
        })
        .to_lowercase();

    let level = input.compression_level.unwrap_or(6).clamp(0, 9);
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;

    if fmt == "tar.gz" || fmt == "tgz" {
        let tar_gz = File::create(&dest_path)
            .with_context(|| format!("Failed to create file {:?}", dest_path))?;
        let enc = GzEncoder::new(tar_gz, Compression::new(level));
        let mut tar = TarBuilder::new(enc);

        for src_str in &input.sources {
            let src = Path::new(src_str);
            if !src.exists() {
                continue;
            }
            let base_parent = src.parent().unwrap_or_else(|| Path::new(""));
            for entry in WalkDir::new(src).follow_links(false) {
                let entry = entry?;
                let path = entry.path();
                let rel = path
                    .strip_prefix(base_parent)
                    .unwrap_or_else(|_| path.file_name().map(Path::new).unwrap_or(path));

                if path.is_file() {
                    let mut f = File::open(path)?;
                    let metadata = entry.metadata()?;
                    tar.append_file(rel, &mut f)?;
                    total_files += 1;
                    total_bytes += metadata.len();
                } else if path.is_dir() && path != src {
                    tar.append_dir(rel, path)?;
                }
            }
        }
        tar.finish()?;
    } else {
        // ZIP format (supports AES-256 password protection)
        let zip_file = File::create(&dest_path)
            .with_context(|| format!("Failed to create file {:?}", dest_path))?;
        let mut zip = ZipWriter::new(zip_file);

        let compression_method = if level == 0 {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };

        for src_str in &input.sources {
            let src = Path::new(src_str);
            if !src.exists() {
                continue;
            }
            let base_parent = src.parent().unwrap_or_else(|| Path::new(""));

            for entry in WalkDir::new(src).follow_links(false) {
                let entry = entry?;
                let path = entry.path();
                let rel = path
                    .strip_prefix(base_parent)
                    .unwrap_or_else(|_| path.file_name().map(Path::new).unwrap_or(path));

                let rel_str = rel.to_string_lossy().replace('\\', "/");

                let mut options = SimpleFileOptions::default()
                    .compression_method(compression_method)
                    .unix_permissions(0o755);

                if let Some(ref pwd) = input.password {
                    if !pwd.is_empty() {
                        options = options.with_aes_encryption(AesMode::Aes256, pwd);
                    }
                }

                if path.is_dir() {
                    let dir_name = if rel_str.ends_with('/') {
                        rel_str
                    } else {
                        format!("{}/", rel_str)
                    };
                    zip.add_directory(&dir_name, options)?;
                } else if path.is_file() {
                    zip.start_file(&rel_str, options)?;
                    let mut f = File::open(path)?;
                    let mut buf = vec![0u8; 64 * 1024];
                    let mut file_len = 0u64;
                    loop {
                        let n = f.read(&mut buf)?;
                        if n == 0 {
                            break;
                        }
                        zip.write_all(&buf[..n])?;
                        file_len += n as u64;
                    }
                    total_files += 1;
                    total_bytes += file_len;
                }
            }
        }
        zip.finish()?;
    }

    Ok(json!({
        "ok": true,
        "destination": dest_path.to_string_lossy().to_string(),
        "totalFiles": total_files,
        "totalBytes": total_bytes
    }))
}

pub fn extract_archive(input: ExtractArchiveIn) -> Result<Value, String> {
    extract_archive_impl(input).map_err(|e| e.to_string())
}

fn extract_archive_impl(input: ExtractArchiveIn) -> Result<Value> {
    let archive_path = PathBuf::from(&input.archive_path);
    if !archive_path.exists() {
        return Err(anyhow!("Archive not found: {:?}", archive_path));
    }

    let dest_dir = PathBuf::from(&input.destination_dir);
    fs::create_dir_all(&dest_dir)
        .with_context(|| format!("Failed to create destination dir {:?}", dest_dir))?;
    let canonical_dest = fs::canonicalize(&dest_dir)
        .with_context(|| format!("Failed to canonicalize destination dir {:?}", dest_dir))?;

    let lower = input.archive_path.to_lowercase();
    let is_tar_gz = lower.ends_with(".tar.gz") || lower.ends_with(".tgz");
    let is_tar = lower.ends_with(".tar");

    let mut extracted_count = 0usize;

    if is_tar_gz || is_tar {
        let f = File::open(&archive_path)?;
        if is_tar_gz {
            let gz = GzDecoder::new(f);
            let mut archive = TarArchive::new(gz);
            for entry_res in archive.entries()? {
                let mut entry = entry_res?;
                let path = entry.path()?.to_path_buf();
                let out_path = safe_resolve_path(&canonical_dest, &path)?;
                if entry.header().entry_type().is_dir() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&out_path)?;
                    extracted_count += 1;
                }
            }
        } else {
            let mut archive = TarArchive::new(f);
            for entry_res in archive.entries()? {
                let mut entry = entry_res?;
                let path = entry.path()?.to_path_buf();
                let out_path = safe_resolve_path(&canonical_dest, &path)?;
                if entry.header().entry_type().is_dir() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&out_path)?;
                    extracted_count += 1;
                }
            }
        }
    } else {
        // ZIP format
        let f = File::open(&archive_path)?;
        let mut archive = ZipArchive::new(f)
            .map_err(|e| anyhow!("Failed to open zip archive: {}", e))?;

        let password = input.password.as_deref().unwrap_or("");

        for i in 0..archive.len() {
            let res = if !password.is_empty() {
                archive.by_index_decrypt(i, password.as_bytes())
            } else {
                archive.by_index(i)
            };

            let mut zip_file = match res {
                Ok(file) => file,
                Err(zip::result::ZipError::InvalidPassword) => {
                    return Err(anyhow!("INVALID_PASSWORD"));
                }
                Err(zip::result::ZipError::UnsupportedArchive(msg))
                    if msg.contains("password") || msg.contains("encrypted") =>
                {
                    return Err(anyhow!("PASSWORD_REQUIRED"));
                }
                Err(e) => return Err(anyhow!("Failed reading zip entry: {}", e)),
            };

            let name = zip_file.name().to_string();
            let out_path = safe_resolve_path(&canonical_dest, Path::new(&name))?;

            if zip_file.is_dir() || name.ends_with('/') {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outf = File::create(&out_path)?;
                std::io::copy(&mut zip_file, &mut outf)?;
                extracted_count += 1;
            }
        }
    }

    Ok(json!({
        "ok": true,
        "extractedCount": extracted_count,
        "destinationDir": canonical_dest.to_string_lossy().to_string()
    }))
}

pub fn list_archive_entries(input: ListArchiveIn) -> Result<Value, String> {
    list_archive_entries_impl(input).map_err(|e| e.to_string())
}

fn list_archive_entries_impl(input: ListArchiveIn) -> Result<Value> {
    let archive_path = PathBuf::from(&input.archive_path);
    if !archive_path.exists() {
        return Err(anyhow!("Archive not found: {:?}", archive_path));
    }

    let lower = input.archive_path.to_lowercase();
    let is_tar_gz = lower.ends_with(".tar.gz") || lower.ends_with(".tgz");
    let is_tar = lower.ends_with(".tar");

    let mut entries: Vec<ArchiveEntryInfo> = Vec::new();

    if is_tar_gz || is_tar {
        let f = File::open(&archive_path)?;
        if is_tar_gz {
            let gz = GzDecoder::new(f);
            let mut archive = TarArchive::new(gz);
            for entry_res in archive.entries()? {
                let entry = entry_res?;
                let path = entry.path()?.to_string_lossy().to_string();
                let is_dir = entry.header().entry_type().is_dir();
                let size = entry.size();
                entries.push(ArchiveEntryInfo {
                    name: path,
                    uncompressed_size: size,
                    compressed_size: size,
                    is_dir,
                    is_encrypted: false,
                });
            }
        } else {
            let mut archive = TarArchive::new(f);
            for entry_res in archive.entries()? {
                let entry = entry_res?;
                let path = entry.path()?.to_string_lossy().to_string();
                let is_dir = entry.header().entry_type().is_dir();
                let size = entry.size();
                entries.push(ArchiveEntryInfo {
                    name: path,
                    uncompressed_size: size,
                    compressed_size: size,
                    is_dir,
                    is_encrypted: false,
                });
            }
        }
    } else {
        let f = File::open(&archive_path)?;
        let mut archive = ZipArchive::new(f)?;
        for i in 0..archive.len() {
            let file = archive.by_index_raw(i)?;
            let name = file.name().to_string();
            let is_dir = file.is_dir() || name.ends_with('/');
            entries.push(ArchiveEntryInfo {
                name,
                uncompressed_size: file.size(),
                compressed_size: file.compressed_size(),
                is_dir,
                is_encrypted: file.encrypted(),
            });
        }
    }

    Ok(json!({
        "ok": true,
        "entries": entries
    }))
}

/// Safely resolves a path against a base canonical directory to prevent Zip-Slip attacks.
fn safe_resolve_path(canonical_base: &Path, rel_path: &Path) -> Result<PathBuf> {
    let mut out = canonical_base.to_path_buf();
    for comp in rel_path.components() {
        match comp {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() || !out.starts_with(canonical_base) {
                    return Err(anyhow!("Archive entry escapes destination directory (zip-slip)"));
                }
            }
            _ => return Err(anyhow!("Invalid entry path component")),
        }
    }
    if !out.starts_with(canonical_base) {
        return Err(anyhow!("Archive entry escapes destination directory (zip-slip)"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_zip_create_and_extract_unencrypted() {
        let dir = tempdir().unwrap();
        let src_file = dir.path().join("hello.txt");
        fs::write(&src_file, "Hello Oryn Archive!").unwrap();

        let zip_dest = dir.path().join("test.zip");
        let res = create_archive_impl(CreateArchiveIn {
            sources: vec![src_file.to_string_lossy().to_string()],
            destination: zip_dest.to_string_lossy().to_string(),
            format: Some("zip".into()),
            password: None,
            compression_level: Some(6),
        }).unwrap();

        assert_eq!(res["ok"], true);
        assert!(zip_dest.exists());

        let extract_dir = dir.path().join("extracted");
        let ext_res = extract_archive_impl(ExtractArchiveIn {
            archive_path: zip_dest.to_string_lossy().to_string(),
            destination_dir: extract_dir.to_string_lossy().to_string(),
            password: None,
        }).unwrap();

        assert_eq!(ext_res["ok"], true);
        let extracted_file = extract_dir.join("hello.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file).unwrap(), "Hello Oryn Archive!");
    }

    #[test]
    fn test_zip_create_and_extract_encrypted() {
        let dir = tempdir().unwrap();
        let src_file = dir.path().join("secret.txt");
        fs::write(&src_file, "Super Secret Data 12345").unwrap();

        let zip_dest = dir.path().join("encrypted.zip");
        create_archive_impl(CreateArchiveIn {
            sources: vec![src_file.to_string_lossy().to_string()],
            destination: zip_dest.to_string_lossy().to_string(),
            format: Some("zip".into()),
            password: Some("mySecretPass99".into()),
            compression_level: Some(6),
        }).unwrap();

        let extract_dir = dir.path().join("extracted_enc");

        // Attempt without password should fail
        let no_pass_res = extract_archive_impl(ExtractArchiveIn {
            archive_path: zip_dest.to_string_lossy().to_string(),
            destination_dir: extract_dir.to_string_lossy().to_string(),
            password: None,
        });
        assert!(no_pass_res.is_err());

        // Extract with correct password
        let ext_res = extract_archive_impl(ExtractArchiveIn {
            archive_path: zip_dest.to_string_lossy().to_string(),
            destination_dir: extract_dir.to_string_lossy().to_string(),
            password: Some("mySecretPass99".into()),
        }).unwrap();

        assert_eq!(ext_res["ok"], true);
        let extracted_file = extract_dir.join("secret.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file).unwrap(), "Super Secret Data 12345");
    }

    #[test]
    fn test_tar_gz_create_and_extract() {
        let dir = tempdir().unwrap();
        let src_file = dir.path().join("tar_test.txt");
        fs::write(&src_file, "Tar Gz Contents").unwrap();

        let tar_dest = dir.path().join("test.tar.gz");
        create_archive_impl(CreateArchiveIn {
            sources: vec![src_file.to_string_lossy().to_string()],
            destination: tar_dest.to_string_lossy().to_string(),
            format: Some("tar.gz".into()),
            password: None,
            compression_level: Some(6),
        }).unwrap();

        let extract_dir = dir.path().join("extracted_tar");
        let ext_res = extract_archive_impl(ExtractArchiveIn {
            archive_path: tar_dest.to_string_lossy().to_string(),
            destination_dir: extract_dir.to_string_lossy().to_string(),
            password: None,
        }).unwrap();

        assert_eq!(ext_res["ok"], true);
        let extracted_file = extract_dir.join("tar_test.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file).unwrap(), "Tar Gz Contents");
    }
}
