use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::services::ServiceResult;

#[derive(Debug, Clone)]
pub struct ListItem {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub mtime: String,
}

pub fn list_dir(path: &Path) -> ServiceResult<Vec<ListItem>> {
    let path_buf;
    let effective_path = {
        let path_str = path.to_string_lossy();
        if path_str.len() == 2
            && path_str.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            && path_str.ends_with(':')
        {
            path_buf = std::path::PathBuf::from(format!("{}\\", path_str));
            &path_buf
        } else {
            path
        }
    };

    let entries = fs::read_dir(effective_path)?;
    let mut dirs: Vec<ListItem> = Vec::new();
    let mut files: Vec<ListItem> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "." {
            continue;
        }

        let is_symlink = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
        let link_meta = entry.metadata().ok();
        let target_meta = if is_symlink {
            fs::metadata(entry.path()).ok()
        } else {
            None
        };
        let effective_meta = target_meta.as_ref().or(link_meta.as_ref());
        let mtime = effective_meta.map(filetime_to_iso).unwrap_or_default();

        let is_dir = target_meta
            .as_ref()
            .map(|m| m.is_dir())
            .or_else(|| link_meta.as_ref().map(|m| m.is_dir()))
            .unwrap_or(false);
        let size = if is_dir {
            None
        } else {
            effective_meta.and_then(|m| Some(m.len()))
        };

        if is_dir {
            dirs.push(ListItem {
                name,
                is_dir: true,
                size,
                mtime,
            });
        } else {
            files.push(ListItem {
                name,
                is_dir: false,
                size,
                mtime,
            });
        }
    }

    dirs.sort_by_key(|item| item.name.to_lowercase());
    files.sort_by_key(|item| item.name.to_lowercase());

    dirs.extend(files);
    Ok(dirs)
}

pub fn list_flat_branch(root: &Path, max_items: usize) -> ServiceResult<Vec<ListItem>> {
    let mut files: Vec<ListItem> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(path);
            let name = rel.to_string_lossy().replace('\\', "/");

            let meta = entry.metadata().ok();
            let mtime = meta.as_ref().map(filetime_to_iso).unwrap_or_default();
            let size = meta.as_ref().map(|m| m.len());

            files.push(ListItem {
                name,
                is_dir: false,
                size,
                mtime,
            });

            if files.len() >= max_items {
                break;
            }
        }
    }

    files.sort_by_key(|item| item.name.to_lowercase());
    Ok(files)
}

fn filetime_to_iso(st: &fs::Metadata) -> String {
    let t = st.modified().unwrap_or(UNIX_EPOCH);
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::list_dir;
    use std::fs;

    #[test]
    fn dirs_first_then_files_sorted_case_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("Zeta")).unwrap();
        fs::create_dir(root.join("alpha")).unwrap();
        fs::write(root.join("b.txt"), b"bb").unwrap();
        fs::write(root.join("A.txt"), b"a").unwrap();

        let items = list_dir(root).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Zeta", "A.txt", "b.txt"]);

        assert!(items[0].is_dir && items[1].is_dir);
        assert!(!items[2].is_dir && !items[3].is_dir);
        assert_eq!(items[2].size, Some(1));
        assert_eq!(items[3].size, Some(2));
        assert_eq!(items[0].size, None, "dirs report no size");
    }

    #[test]
    fn missing_dir_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("nope");
        assert!(list_dir(&gone).is_err());
    }

    #[test]
    fn directory_symlinks_are_listed_as_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let target_dir = tmp.path().join("real_folder");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("inner.txt"), b"hi").unwrap();

        let link = tmp.path().join("link_folder");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_dir, &link).unwrap();
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_dir(&target_dir, &link);

        let items = list_dir(tmp.path()).unwrap();
        let link_item = items.iter().find(|i| i.name == "link_folder");
        if let Some(item) = link_item {
            assert!(item.is_dir, "Symlink to a directory must be listed as is_dir: true");
        }
    }
}
