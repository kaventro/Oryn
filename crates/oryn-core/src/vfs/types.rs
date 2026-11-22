use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub display: String,
    pub base: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    pub size: Option<u64>,
    pub mtime: String,
}

pub trait VirtualFileSystem: Send + Sync {
    fn read_dir(&self, path: &str) -> Result<Vec<FileItem>>;

    fn extract_to(&self, _src_path: &str, _dst: &std::path::Path) -> Result<()> {
        Err(anyhow::anyhow!(
            "extract_to is not supported by this provider"
        ))
    }
}

pub(crate) const MAX_VFS_EXTRACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
