use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

pub const READ_CHUNK: usize = 1024 * 1024;
pub const MAX_JS_DIFF: u64 = 8 * 1024 * 1024;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEnt {
    pub rel: String,
    pub full: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: f64,
    pub dev: u64,
    pub ino: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

pub fn filetime_ms(meta: &fs::Metadata) -> f64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

pub fn build_dir_map(root: &Path) -> Result<HashMap<String, DirEnt>, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for e in walkdir::WalkDir::new(&root)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !e.file_type().is_file() && !e.file_type().is_dir() {
            continue;
        }
        let full = e.path().to_path_buf();
        let rel_s = full
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| {
                full.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
        if rel_s.is_empty() {
            continue;
        }
        let meta = match fs::metadata(&full) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let (dev, ino) = file_ids(&meta);
        map.insert(
            rel_s.clone(),
            DirEnt {
                rel: rel_s,
                full: full.to_string_lossy().to_string(),
                is_dir,
                size: if is_dir { 0 } else { meta.len() },
                mtime: filetime_ms(&meta),
                dev,
                ino,
                side: None,
            },
        );
    }
    Ok(map)
}

#[cfg(unix)]
fn file_ids(meta: &fs::Metadata) -> (u64, String) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino().to_string())
}

#[cfg(not(unix))]
fn file_ids(_meta: &fs::Metadata) -> (u64, String) {
    (0, "0".into())
}

pub fn same_inode(a: &DirEnt, b: &DirEnt) -> bool {
    a.dev == b.dev && a.ino != "0" && a.ino == b.ino
}

pub fn binary_differ(l: &Path, r: &Path) -> Result<bool, String> {
    let mut f1 = fs::File::open(l).map_err(|e| e.to_string())?;
    let mut f2 = fs::File::open(r).map_err(|e| e.to_string())?;
    let mut b1 = vec![0u8; READ_CHUNK];
    let mut b2 = vec![0u8; READ_CHUNK];
    loop {
        let n1 = f1.read(&mut b1).map_err(|e| e.to_string())?;
        let n2 = f2.read(&mut b2).map_err(|e| e.to_string())?;
        if n1 != n2 {
            return Ok(true);
        }
        if n1 == 0 {
            return Ok(false);
        }
        if b1[..n1] != b2[..n2] {
            return Ok(true);
        }
    }
}

pub fn buffer_looks_binary(buf: &[u8]) -> bool {
    buf.iter()
        .take(8192)
        .any(|&c| c == 0 || (c < 8 && c != 9 && c != 10 && c != 13))
}

pub fn sort_rows(mut v: Vec<Value>) -> Vec<Value> {
    v.sort_by(|a, b| {
        let ra = a.get("rel").and_then(|x| x.as_str()).unwrap_or("");
        let rb = b.get("rel").and_then(|x| x.as_str()).unwrap_or("");
        ra.cmp(rb)
    });
    v
}
