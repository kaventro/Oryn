use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use super::session::RemoteSession;
use crate::services::fs_vfs::DirListing;
use crate::vfs::types::FileItem;

pub fn list_dir(session: &RemoteSession, remote_path: &str) -> Result<DirListing> {
    let sftp = session.sftp();
    let norm_path = if remote_path.is_empty() {
        "/"
    } else {
        remote_path
    };

    let p = Path::new(norm_path);
    let mut entries = sftp
        .readdir(p)
        .with_context(|| format!("Failed to read remote directory: {}", norm_path))?;

    let mut items: Vec<FileItem> = Vec::with_capacity(entries.len() + 1);

    // Add parent ".." entry
    items.push(FileItem {
        display: "..".into(),
        base: "..".into(),
        is_dir: true,
        size: None,
        mtime: String::new(),
    });

    let mut file_items = Vec::new();

    for (path, stat) in entries.drain(..) {
        let filename = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        if filename == "." || filename == ".." {
            continue;
        }

        let is_dir = stat.is_dir();
        let size = if is_dir { None } else { stat.size };
        let mtime = stat
            .mtime
            .map(|t| {
                DateTime::<Utc>::from_timestamp(t as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        file_items.push(FileItem {
            display: filename.clone(),
            base: filename,
            is_dir,
            size,
            mtime,
        });
    }

    // Sort: directories first, then alphabetical (case-insensitive)
    file_items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.base.to_lowercase().cmp(&b.base.to_lowercase()),
    });

    items.extend(file_items);

    Ok(DirListing { ok: true, items })
}

pub fn read_file_text(
    session: &RemoteSession,
    remote_path: &str,
    max_bytes: usize,
) -> Result<String> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    let mut file = sftp
        .open(path)
        .with_context(|| format!("Failed to open remote file: {}", remote_path))?;

    let mut buf = Vec::new();
    let mut take = (&mut file).take(max_bytes as u64);
    take.read_to_end(&mut buf)
        .with_context(|| format!("Failed to read remote file content: {}", remote_path))?;

    String::from_utf8(buf).map_err(|_| anyhow!("File is not valid UTF-8 text"))
}

pub fn write_file_text(session: &RemoteSession, remote_path: &str, content: &str) -> Result<()> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    let mut file = sftp
        .create(path)
        .with_context(|| format!("Failed to create remote file: {}", remote_path))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write to remote file: {}", remote_path))?;
    Ok(())
}

pub fn mkdir(session: &RemoteSession, remote_path: &str) -> Result<()> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    sftp.mkdir(path, 0o755)
        .with_context(|| format!("Failed to create remote directory: {}", remote_path))?;
    Ok(())
}

pub fn create_file(session: &RemoteSession, remote_path: &str) -> Result<()> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    let _ = sftp
        .create(path)
        .with_context(|| format!("Failed to create remote file: {}", remote_path))?;
    Ok(())
}

pub fn rename(session: &RemoteSession, src_path: &str, dst_path: &str) -> Result<()> {
    let sftp = session.sftp();
    sftp.rename(Path::new(src_path), Path::new(dst_path), None)
        .with_context(|| {
            format!(
                "Failed to rename remote path from {} to {}",
                src_path, dst_path
            )
        })?;
    Ok(())
}

pub fn delete_file(session: &RemoteSession, remote_path: &str) -> Result<()> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    sftp.unlink(path)
        .with_context(|| format!("Failed to delete remote file: {}", remote_path))?;
    Ok(())
}

pub fn delete_dir_recursive(session: &RemoteSession, remote_path: &str) -> Result<()> {
    let sftp = session.sftp();
    let path = Path::new(remote_path);
    if let Ok(entries) = sftp.readdir(path) {
        for (sub_path, stat) in entries {
            let fname = sub_path.file_name().unwrap_or_default().to_string_lossy();
            if fname == "." || fname == ".." {
                continue;
            }
            let sub_path_str = sub_path.to_string_lossy().to_string();
            if stat.is_dir() {
                let _ = delete_dir_recursive(session, &sub_path_str);
            } else {
                let _ = sftp.unlink(&sub_path);
            }
        }
    }
    sftp.rmdir(path)
        .with_context(|| format!("Failed to remove remote directory: {}", remote_path))?;
    Ok(())
}

pub fn download_file(
    session: &RemoteSession,
    remote_path: &str,
    local_dst_path: &Path,
) -> Result<u64> {
    let sftp = session.sftp();
    let mut remote_file = sftp
        .open(Path::new(remote_path))
        .with_context(|| format!("Failed to open remote file: {}", remote_path))?;

    if let Some(parent) = local_dst_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut local_file = File::create(local_dst_path).with_context(|| {
        format!(
            "Failed to create local destination: {}",
            local_dst_path.display()
        )
    })?;

    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;

    loop {
        let n = remote_file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        local_file.write_all(&buffer[..n])?;
        total += n as u64;
    }

    Ok(total)
}

pub fn upload_file(
    session: &RemoteSession,
    local_src_path: &Path,
    remote_dst_path: &str,
) -> Result<u64> {
    let sftp = session.sftp();
    let mut local_file = File::open(local_src_path).with_context(|| {
        format!(
            "Failed to open local source file: {}",
            local_src_path.display()
        )
    })?;

    let mut remote_file = sftp
        .create(Path::new(remote_dst_path))
        .with_context(|| format!("Failed to create remote destination: {}", remote_dst_path))?;

    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;

    loop {
        let n = local_file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        remote_file.write_all(&buffer[..n])?;
        total += n as u64;
    }

    Ok(total)
}
