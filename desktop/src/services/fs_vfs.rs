use serde::Serialize;

use crate::services::ServiceResult;
use crate::vfs::{FileItem, VfsRouter, VirtualFileSystem};

#[derive(Serialize)]
pub struct DirListing {
    pub ok: bool,
    pub items: Vec<FileItem>,
}

pub fn read_dir_response(path: &str) -> ServiceResult<DirListing> {
    let router = VfsRouter::new();
    let listed = router.read_dir(path)?;

    let mut items = Vec::with_capacity(listed.len() + 1);
    items.push(FileItem {
        display: "..".into(),
        base: "..".into(),
        is_dir: true,
        size: None,
        mtime: String::new(),
    });
    items.extend(listed);

    Ok(DirListing { ok: true, items })
}

#[cfg(test)]
mod tests {
    use super::read_dir_response;
    use std::io::Write;

    #[test]
    fn zip_path_lists_like_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("demo.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        writer.start_file("docs/a.md", opts).unwrap();
        writer.write_all(b"x").unwrap();
        writer.start_file("top.txt", opts).unwrap();
        writer.write_all(b"yy").unwrap();
        writer.finish().unwrap();

        let listing = read_dir_response(zip_path.to_str().unwrap()).unwrap();
        assert!(listing.ok);
        let names: Vec<&str> = listing.items.iter().map(|i| i.base.as_str()).collect();
        assert_eq!(names, vec!["..", "docs", "top.txt"]);
    }

    #[test]
    fn plain_dir_still_lists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("f.txt"), b"1").unwrap();
        let listing = read_dir_response(tmp.path().to_str().unwrap()).unwrap();
        assert!(listing.ok);
        assert_eq!(listing.items.len(), 2);
    }
}
