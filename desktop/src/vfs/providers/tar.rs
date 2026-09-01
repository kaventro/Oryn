use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use std::fs::File;
use tar::Archive;

use crate::vfs::types::{FileItem, VirtualFileSystem};

fn split_tar_path(full_path: &str) -> Option<(String, String)> {
    let lower = full_path.to_lowercase();
    let extensions = [
        ".tar.gz/",
        ".tar.gz\\",
        ".tar.gz",
        ".tar.bz2/",
        ".tar.bz2\\",
        ".tar.bz2",
        ".tar.xz/",
        ".tar.xz\\",
        ".tar.xz",
        ".tar.zst/",
        ".tar.zst\\",
        ".tar.zst",
        ".tgz/",
        ".tgz\\",
        ".tgz",
        ".tar/",
        ".tar\\",
        ".tar",
    ];

    for ext in extensions {
        if let Some(idx) = lower.find(ext) {
            let clean_ext = ext.trim_end_matches(['/', '\\']);
            let split_idx = idx + clean_ext.len();
            let host_path = &full_path[..split_idx];
            let inner_path = if full_path.len() > split_idx {
                let rest = &full_path[split_idx..];
                rest.trim_start_matches(['/', '\\'])
            } else {
                ""
            };
            return Some((host_path.to_string(), inner_path.replace('\\', "/")));
        }
    }

    None
}

pub struct TarProvider;

impl VirtualFileSystem for TarProvider {
    fn read_dir(&self, path: &str) -> Result<Vec<FileItem>> {
        let (host_path, mut inner_path) =
            split_tar_path(path).ok_or_else(|| anyhow!("Not a tar path"))?;

        inner_path = inner_path.trim_matches('/').to_string();
        let inner_prefix = if inner_path.is_empty() {
            "".to_string()
        } else {
            format!("{}/", inner_path)
        };

        let file = File::open(&host_path)?;
        let mut archive = if host_path.ends_with(".gz") || host_path.ends_with(".tgz") {
            Archive::new(Box::new(GzDecoder::new(file)) as Box<dyn std::io::Read>)
        } else {
            Archive::new(Box::new(file) as Box<dyn std::io::Read>)
        };

        let mut dirs: Vec<FileItem> = Vec::new();
        let mut files: Vec<FileItem> = Vec::new();
        let mut seen_dirs = std::collections::HashSet::new();

        for entry in archive.entries()? {
            let entry = entry?;
            let path_buf = entry.path()?;
            let name = path_buf.to_string_lossy().to_string();

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
                } else {
                    let is_dir = entry.header().entry_type().is_dir();
                    let base_name = remainder.to_string();

                    if is_dir {
                        if !seen_dirs.contains(&base_name) {
                            seen_dirs.insert(base_name.clone());
                            dirs.push(FileItem {
                                display: format!("/{}", base_name),
                                base: base_name,
                                is_dir: true,
                                size: None,
                                mtime: "".to_string(),
                            });
                        }
                    } else {
                        files.push(FileItem {
                            display: base_name.clone(),
                            base: base_name,
                            is_dir: false,
                            size: Some(entry.header().size()?),
                            mtime: "".to_string(),
                        });
                    }
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
            split_tar_path(src_path).ok_or_else(|| anyhow!("Not a tar path"))?;

        inner_path = inner_path.trim_matches('/').to_string();
        let file = File::open(&host_path)?;

        let mut archive = if host_path.ends_with(".gz") || host_path.ends_with(".tgz") {
            Archive::new(Box::new(GzDecoder::new(file)) as Box<dyn std::io::Read>)
        } else {
            Archive::new(Box::new(file) as Box<dyn std::io::Read>)
        };

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path_buf = entry.path()?;
            if path_buf.to_string_lossy() == inner_path {
                let parent = dst.parent().unwrap_or_else(|| std::path::Path::new("."));
                std::fs::create_dir_all(parent)?;
                let name = dst
                    .file_name()
                    .ok_or_else(|| anyhow!("Invalid destination file name"))?;
                let root = crate::fs_safe::SafeRoot::open(parent)?;
                root.write_file(
                    std::path::Path::new(name),
                    &mut entry,
                    crate::vfs::types::MAX_VFS_EXTRACT_BYTES,
                    None,
                )?;
                return Ok(());
            }
        }

        Err(anyhow!("File not found in tar archive"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::types::VirtualFileSystem;

    fn make_targz(dir: &std::path::Path) -> std::path::PathBuf {
        let tar_path = dir.join("demo.tar.gz");
        let file = File::create(&tar_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(5);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "readme.txt", &b"hello"[..])
            .unwrap();

        let mut header2 = tar::Header::new_gnu();
        header2.set_size(3);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder
            .append_data(&mut header2, "docs/a.md", &b"abc"[..])
            .unwrap();

        builder.into_inner().unwrap().finish().unwrap();
        tar_path
    }

    #[test]
    fn lists_targz_root() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_path = make_targz(tmp.path());
        let items = TarProvider.read_dir(tar_path.to_str().unwrap()).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(names, vec!["docs", "readme.txt"]);
        assert!(items[0].is_dir);
        assert_eq!(items[1].size, Some(5));
    }

    #[test]
    fn lists_targz_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_path = make_targz(tmp.path());
        let inner = format!("{}/docs", tar_path.to_str().unwrap());
        let items = TarProvider.read_dir(&inner).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(names, vec!["a.md"]);
    }
}
