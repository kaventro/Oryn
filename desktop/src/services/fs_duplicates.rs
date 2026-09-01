// desktop/src/services/fs_duplicates.rs
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub mtime: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub files: Vec<DuplicateFile>,
    pub total_wasted: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanResult {
    pub ok: bool,
    pub total_scanned: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub total_wasted_bytes: u64,
    pub duplicate_files_count: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanOptions {
    pub path: String,
    pub min_size_bytes: Option<u64>,
    pub max_results: Option<usize>,
}

pub fn scan_duplicates(opts: DuplicateScanOptions) -> Result<DuplicateScanResult> {
    let root = Path::new(&opts.path);
    if !root.exists() {
        return Err(anyhow::anyhow!("Directory does not exist: {}", opts.path));
    }

    let min_size = opts.min_size_bytes.unwrap_or(1024); // default > 1 KB
    let mut total_scanned = 0u64;

    // Step 1: Collect files and group by exact length
    let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            total_scanned += 1;
            if let Ok(meta) = entry.metadata() {
                let len = meta.len();
                if len >= min_size {
                    size_map.entry(len).or_default().push(entry.into_path());
                }
            }
        }
    }

    // Filter sizes with at least 2 files
    let candidate_items: Vec<(u64, PathBuf)> = size_map
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .flat_map(|(size, paths)| paths.into_iter().map(move |p| (size, p)))
        .collect();

    // Step 2 & 3: Compute hashes in parallel using bounded worker pool
    let mut hash_map: HashMap<(u64, String), Vec<PathBuf>> = HashMap::new();

    if !candidate_items.is_empty() {
        let queue = Mutex::new(candidate_items);
        let results = Mutex::new(Vec::new());

        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(8);

        std::thread::scope(|scope| {
            for _ in 0..workers {
                let queue = &queue;
                let results = &results;
                scope.spawn(move || loop {
                    let next = {
                        let mut guard = queue.lock().unwrap();
                        guard.pop()
                    };
                    let Some((size, path)) = next else {
                        break;
                    };
                    if let Ok(hash) = compute_file_hash(&path) {
                        let mut guard = results.lock().unwrap();
                        guard.push((size, hash, path));
                    }
                });
            }
        });

        let hashed_files = results.into_inner().unwrap();
        for (size, hash, path) in hashed_files {
            hash_map.entry((size, hash)).or_default().push(path);
        }
    }

    let mut duplicate_groups = Vec::new();
    let mut total_wasted_bytes = 0u64;
    let mut duplicate_files_count = 0u64;

    for ((size, hash), paths) in hash_map {
        if paths.len() > 1 {
            let files: Vec<DuplicateFile> = paths
                .iter()
                .map(|p| {
                    let name = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let mtime = p
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let secs = t
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;
                            chrono::DateTime::from_timestamp(secs, 0)
                                .map(|d| d.to_rfc3339())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    DuplicateFile {
                        path: p.to_string_lossy().to_string(),
                        name,
                        size,
                        mtime,
                    }
                })
                .collect();

            let count = files.len() as u64;
            let wasted = (count - 1) * size;
            total_wasted_bytes += wasted;
            duplicate_files_count += count;

            duplicate_groups.push(DuplicateGroup {
                hash,
                size,
                files,
                total_wasted: wasted,
            });
        }
    }

    // Sort by wasted bytes descending (heaviest duplicate sets first)
    duplicate_groups.sort_by(|a, b| b.total_wasted.cmp(&a.total_wasted));

    if let Some(max) = opts.max_results {
        if duplicate_groups.len() > max {
            duplicate_groups.truncate(max);
        }
    }

    Ok(DuplicateScanResult {
        ok: true,
        total_scanned,
        duplicate_groups,
        total_wasted_bytes,
        duplicate_files_count,
    })
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let meta = file.metadata()?;
    let len = meta.len();

    let mut hasher = Sha256::new();

    if len > 2 * 1024 * 1024 {
        let chunk_size = 64 * 1024;
        let mut buf = vec![0u8; chunk_size];

        // Head
        let n = file.read(&mut buf)?;
        hasher.update(&buf[..n]);

        // Middle
        if len > 3 * chunk_size as u64 {
            file.seek(SeekFrom::Start(len / 2))?;
            let n = file.read(&mut buf)?;
            hasher.update(&buf[..n]);
        }

        // Tail
        if len > chunk_size as u64 {
            file.seek(SeekFrom::End(-(chunk_size as i64)))?;
            let n = file.read(&mut buf)?;
            hasher.update(&buf[..n]);
        }

        hasher.update(len.to_le_bytes());
    } else {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        hasher.update(&buf);
    }

    let hash_bytes = hasher.finalize();
    Ok(hex::encode(hash_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let content_a = b"Exact duplicate content here 1234567890";
        let content_b = b"Different unique file contents";

        fs::write(tmp.path().join("file1.txt"), content_a).unwrap();
        fs::write(tmp.path().join("file2.txt"), content_a).unwrap();
        fs::write(tmp.path().join("file3.txt"), content_b).unwrap();

        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("file1_copy.txt"), content_a).unwrap();

        let res = scan_duplicates(DuplicateScanOptions {
            path: tmp.path().to_string_lossy().to_string(),
            min_size_bytes: Some(10),
            max_results: None,
        })
        .unwrap();

        assert!(res.ok);
        assert_eq!(res.total_scanned, 4);
        assert_eq!(res.duplicate_groups.len(), 1);
        assert_eq!(res.duplicate_groups[0].files.len(), 3);
        assert_eq!(res.duplicate_files_count, 3);
    }
}
