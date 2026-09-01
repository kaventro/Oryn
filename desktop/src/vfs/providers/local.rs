use anyhow::Result;

use crate::services::fs_listing;
use crate::vfs::types::{FileItem, VirtualFileSystem};

pub struct LocalProvider;

impl VirtualFileSystem for LocalProvider {
    fn read_dir(&self, path: &str) -> Result<Vec<FileItem>> {
        let listed = fs_listing::list_dir(std::path::Path::new(path))?;
        Ok(listed
            .into_iter()
            .map(|item| FileItem {
                display: if item.is_dir {
                    format!("/{}", item.name)
                } else {
                    item.name.clone()
                },
                base: item.name,
                is_dir: item.is_dir,
                size: item.size,
                mtime: item.mtime,
            })
            .collect())
    }

    fn extract_to(&self, src_path: &str, dst: &std::path::Path) -> Result<()> {
        std::fs::copy(src_path, dst)?;
        Ok(())
    }
}
