use anyhow::{anyhow, Result};
use std::fs::{self, File};
use zip::ZipArchive;

use crate::vfs::types::{FileItem, VirtualFileSystem};

fn split_zip_path(full_path: &str) -> Option<(String, String)> {
    let lower = full_path.to_lowercase();
    if let Some(idx) = lower.find(".zip/").or_else(|| lower.find(".zip\\")) {
        let split_idx = idx + 4;
        let host_path = &full_path[..split_idx];
        let inner_path = &full_path[split_idx + 1..];
        Some((host_path.to_string(), inner_path.replace('\\', "/")))
    } else if lower.ends_with(".zip") {
        Some((full_path.to_string(), "".to_string()))
    } else {
        None
    }
}

fn entry_mtime_iso<R: std::io::Read + ?Sized>(file: &zip::read::ZipFile<'_, R>) -> String {
    file.last_modified()
        .map(|dt| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )
        })
        .unwrap_or_default()
}

pub struct ZipProvider;

impl VirtualFileSystem for ZipProvider {
    fn read_dir(&self, path: &str) -> Result<Vec<FileItem>> {
        let (host_path, mut inner_path) =
            split_zip_path(path).ok_or_else(|| anyhow!("Not a zip path"))?;

        inner_path = inner_path.trim_matches('/').to_string();

        let inner_prefix = if inner_path.is_empty() {
            "".to_string()
        } else {
            format!("{}/", inner_path)
        };

        let file = File::open(&host_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut dirs: Vec<FileItem> = Vec::new();
        let mut files: Vec<FileItem> = Vec::new();
        let mut seen_dirs = std::collections::HashSet::new();

        for index in 0..archive.len() {
            let file = archive.by_index(index)?;
            let name = file.name().to_string();

            if name.starts_with(&inner_prefix) && name != inner_prefix {
                let remainder = &name[inner_prefix.len()..];
                if remainder.is_empty() {
                    continue;
                }

                if let Some(slash_idx) = remainder.find('/') {
                    let dir_name = remainder[..slash_idx].to_string();
                    if !seen_dirs.contains(&dir_name) {
                        seen_dirs.insert(dir_name.clone());
                        dirs.push(FileItem {
                            display: format!("/{}", dir_name),
                            base: dir_name,
                            is_dir: true,
                            size: None,
                            mtime: "".to_string(),
                        });
                    }
                } else if file.is_dir() {
                    let dir_name = remainder.trim_end_matches('/').to_string();
                    if !seen_dirs.contains(&dir_name) {
                        seen_dirs.insert(dir_name.clone());
                        dirs.push(FileItem {
                            display: format!("/{}", dir_name),
                            base: dir_name,
                            is_dir: true,
                            size: None,
                            mtime: "".to_string(),
                        });
                    }
                } else {
                    files.push(FileItem {
                        display: remainder.to_string(),
                        base: remainder.to_string(),
                        is_dir: false,
                        size: Some(file.size()),
                        mtime: entry_mtime_iso(&file),
                    });
                }
            }
        }

        dirs.sort_by_key(|item| item.base.to_lowercase());
        files.sort_by_key(|item| item.base.to_lowercase());
        dirs.extend(files);

        Ok(dirs)
    }

    fn extract_to(&self, src_path: &str, dst: &std::path::Path) -> Result<()> {
        let (host_path, mut inner_path) =
            split_zip_path(src_path).ok_or_else(|| anyhow!("Not a zip path"))?;

        inner_path = inner_path.trim_matches('/').to_string();

        let file = File::open(&host_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut zip_file = archive.by_name(&inner_path)?;
        if zip_file.is_dir() {
            return Err(anyhow!(
                "Cannot extract a directory using extract_to directly"
            ));
        }

        let parent = dst.parent().unwrap_or_else(|| std::path::Path::new("."));
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
        let name = dst
            .file_name()
            .ok_or_else(|| anyhow!("Invalid destination file name"))?;

        let root = crate::fs_safe::SafeRoot::open(parent)?;
        root.write_file(
            std::path::Path::new(name),
            &mut zip_file,
            crate::vfs::types::MAX_VFS_EXTRACT_BYTES,
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::types::VirtualFileSystem;
    use std::io::Write;

    fn make_zip(dir: &std::path::Path) -> std::path::PathBuf {
        let zip_path = dir.join("test.zip");
        let file = File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("readme.txt", opts).unwrap();
        writer.write_all(b"hello").unwrap();
        writer.start_file("docs/guide.md", opts).unwrap();
        writer.write_all(b"# guide").unwrap();
        writer.start_file("docs/img/logo.png", opts).unwrap();
        writer.write_all(&[1u8, 2, 3]).unwrap();
        writer.finish().unwrap();
        zip_path
    }

    #[test]
    fn lists_zip_root() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_zip(tmp.path());
        let items = ZipProvider.read_dir(zip_path.to_str().unwrap()).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(names, vec!["docs", "readme.txt"]);
        assert!(items[0].is_dir);
        assert_eq!(items[1].size, Some(5));
    }

    #[test]
    fn lists_zip_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_zip(tmp.path());
        let inner = format!("{}/docs", zip_path.to_str().unwrap());
        let items = ZipProvider.read_dir(&inner).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(names, vec!["img", "guide.md"]);
    }

    #[test]
    fn extracts_single_file() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_zip(tmp.path());
        let src = format!("{}/readme.txt", zip_path.to_str().unwrap());
        let dst = tmp.path().join("out.txt");
        ZipProvider.extract_to(&src, &dst).unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello");
    }
}
