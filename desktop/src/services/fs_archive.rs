use anyhow::{anyhow, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

pub fn compress_zip(sources: &[PathBuf], destination_zip: &Path) -> Result<()> {
    if sources.is_empty() {
        return Err(anyhow!("No files selected for compression"));
    }

    if let Some(parent) = destination_zip.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(destination_zip)?;
    let mut zip_writer = zip::write::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut buffer = vec![0u8; 64 * 1024];

    for src in sources {
        if !src.exists() {
            continue;
        }

        let base_name = src
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "item".to_string());

        if src.is_file() {
            zip_writer.start_file(&base_name, options)?;
            let mut f = File::open(src)?;
            loop {
                let count = f.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                zip_writer.write_all(&buffer[..count])?;
            }
        } else if src.is_dir() {
            let parent_dir = src.parent().unwrap_or(src);
            for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                let Ok(rel_path) = path.strip_prefix(parent_dir) else {
                    continue;
                };

                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                if rel_str.is_empty() {
                    continue;
                }

                if entry.file_type().is_dir() {
                    let dir_entry = if rel_str.ends_with('/') {
                        rel_str
                    } else {
                        format!("{rel_str}/")
                    };
                    zip_writer.add_directory(&dir_entry, options)?;
                } else if entry.file_type().is_file() {
                    zip_writer.start_file(&rel_str, options)?;
                    let mut f = File::open(path)?;
                    loop {
                        let count = f.read(&mut buffer)?;
                        if count == 0 {
                            break;
                        }
                        zip_writer.write_all(&buffer[..count])?;
                    }
                }
            }
        }
    }

    zip_writer.finish()?;
    Ok(())
}

const MAX_EXTRACT_ENTRIES: usize = 50_000;
const MAX_TOTAL_EXTRACT_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5 GB
const MAX_SINGLE_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GB

pub fn extract_archive(archive_path: &Path, destination_dir: &Path) -> Result<()> {
    if !archive_path.exists() {
        return Err(anyhow!("Archive not found: {:?}", archive_path));
    }

    fs::create_dir_all(destination_dir)?;
    let root = crate::fs_safe::SafeRoot::open(destination_dir)?;
    let path_str = archive_path.to_string_lossy().to_lowercase();

    if path_str.ends_with(".zip") {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        if archive.len() > MAX_EXTRACT_ENTRIES {
            return Err(anyhow!(
                "Archive exceeds maximum allowed entries limit ({})",
                MAX_EXTRACT_ENTRIES
            ));
        }

        let mut total: u64 = 0;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let rel = match file.enclosed_name() {
                Some(p) => p.to_owned(),
                None => continue,
            };

            if file.is_dir() || file.name().ends_with('/') {
                root.create_dir_all(&rel)?;
            } else {
                #[cfg(unix)]
                let mode = file.unix_mode();
                #[cfg(not(unix))]
                let mode: Option<u32> = None;

                total += root.write_file(&rel, &mut file, MAX_SINGLE_ENTRY_BYTES, mode)?;
                if total > MAX_TOTAL_EXTRACT_BYTES {
                    return Err(anyhow!("Archive total extracted size exceeds safety limit"));
                }
            }
        }
        Ok(())
    } else if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
        let tar_gz = File::open(archive_path)?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tar_gz));
        extract_tar_entries(&mut archive, &root)
    } else if path_str.ends_with(".tar") {
        let file = File::open(archive_path)?;
        let mut archive = tar::Archive::new(file);
        extract_tar_entries(&mut archive, &root)
    } else {
        Err(anyhow!("Unsupported archive format: {:?}", archive_path))
    }
}

fn extract_tar_entries<R: Read>(
    archive: &mut tar::Archive<R>,
    root: &crate::fs_safe::SafeRoot,
) -> Result<()> {
    let mut entry_count = 0usize;
    let mut total: u64 = 0;

    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        entry_count += 1;
        if entry_count > MAX_EXTRACT_ENTRIES {
            return Err(anyhow!(
                "Tar archive exceeds entry count limit ({})",
                MAX_EXTRACT_ENTRIES
            ));
        }

        let rel = entry.path()?.to_path_buf();
        let entry_type = entry.header().entry_type();

        if entry_type.is_dir() {
            root.create_dir_all(&rel)?;
        } else if entry_type.is_file() {
            #[cfg(unix)]
            let mode = entry.header().mode().ok().map(|m| m & 0o7777);
            #[cfg(not(unix))]
            let mode: Option<u32> = None;

            total += root.write_file(&rel, &mut entry, MAX_SINGLE_ENTRY_BYTES, mode)?;
            if total > MAX_TOTAL_EXTRACT_BYTES {
                return Err(anyhow!(
                    "Tar archive total extracted size exceeds safety limit"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_zip_compress_and_extract() {
        let tmp = tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("hello.txt"), b"Hello World").unwrap();
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("sub").join("nested.txt"), b"Nested content").unwrap();

        let zip_path = tmp.path().join("output.zip");
        compress_zip(&[src_dir], &zip_path).unwrap();
        assert!(zip_path.exists());
        assert!(zip_path.metadata().unwrap().len() > 0);

        let out_dir = tmp.path().join("extracted");
        extract_archive(&zip_path, &out_dir).unwrap();

        assert_eq!(
            fs::read(out_dir.join("src").join("hello.txt")).unwrap(),
            b"Hello World"
        );
        assert_eq!(
            fs::read(out_dir.join("src").join("sub").join("nested.txt")).unwrap(),
            b"Nested content"
        );
    }
}
