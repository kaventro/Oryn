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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_local_provider_read_dir_and_extract() {
        let tmp = tempdir().unwrap();
        let sub = tmp.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        let file = tmp.path().join("test.txt");
        std::fs::write(&file, b"sample text").unwrap();

        let items = LocalProvider.read_dir(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(items.len(), 2);
        let names: Vec<&str> = items.iter().map(|i| i.base.as_str()).collect();
        assert!(names.contains(&"subdir"));
        assert!(names.contains(&"test.txt"));

        let dir_item = items.iter().find(|i| i.is_dir).unwrap();
        assert_eq!(dir_item.display, "/subdir");

        // Test extract_to
        let copy_dest = tmp.path().join("copied.txt");
        LocalProvider.extract_to(file.to_str().unwrap(), &copy_dest).unwrap();
        assert_eq!(std::fs::read(&copy_dest).unwrap(), b"sample text");
    }
}
