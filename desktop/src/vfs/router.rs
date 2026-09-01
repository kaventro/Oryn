use anyhow::Result;

use crate::vfs::providers::{local::LocalProvider, tar::TarProvider, zip::ZipProvider};
use crate::vfs::types::{FileItem, VirtualFileSystem};

pub struct VfsRouter {
    local: LocalProvider,
    zip: ZipProvider,
    tar: TarProvider,
}

impl VfsRouter {
    pub fn new() -> Self {
        Self {
            local: LocalProvider,
            zip: ZipProvider,
            tar: TarProvider,
        }
    }

    fn provider(&self, path: &str) -> &dyn VirtualFileSystem {
        match route_kind(path) {
            RouteKind::Zip => &self.zip,
            RouteKind::Tar => &self.tar,
            RouteKind::Local => &self.local,
        }
    }
}

impl Default for VfsRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteKind {
    Zip,
    Tar,
    Local,
}

pub(crate) fn route_kind(path: &str) -> RouteKind {
    let lower = path.to_lowercase();
    if lower.contains(".zip/") || lower.ends_with(".zip") {
        RouteKind::Zip
    } else if is_tar_path(&lower) {
        RouteKind::Tar
    } else {
        RouteKind::Local
    }
}

fn is_tar_path(lower: &str) -> bool {
    let tar_exts = [
        ".tar/",
        ".tar.gz/",
        ".tar.bz2/",
        ".tar.xz/",
        ".tar.zst/",
        ".tgz/",
    ];
    let tar_ends = [".tar", ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".tgz"];
    tar_exts.iter().any(|ext| lower.contains(ext))
        || tar_ends.iter().any(|ext| lower.ends_with(ext))
}

impl VirtualFileSystem for VfsRouter {
    fn read_dir(&self, path: &str) -> Result<Vec<FileItem>> {
        self.provider(path).read_dir(path)
    }

    fn extract_to(&self, src_path: &str, dst: &std::path::Path) -> Result<()> {
        self.provider(src_path).extract_to(src_path, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::{route_kind, RouteKind};

    #[test]
    fn plain_paths_are_local() {
        assert_eq!(route_kind("/Users/me/Documents"), RouteKind::Local);
        assert_eq!(route_kind("/tmp/file.json"), RouteKind::Local);
        assert_eq!(route_kind("C:\\Users\\me"), RouteKind::Local);
    }

    #[test]
    fn zip_paths_route_to_zip() {
        assert_eq!(route_kind("/tmp/archive.zip"), RouteKind::Zip);
        assert_eq!(route_kind("/tmp/Archive.ZIP"), RouteKind::Zip);
        assert_eq!(route_kind("/tmp/archive.zip/inner/dir"), RouteKind::Zip);
    }

    #[test]
    fn tar_paths_route_to_tar() {
        assert_eq!(route_kind("/tmp/a.tar"), RouteKind::Tar);
        assert_eq!(route_kind("/tmp/a.tar.gz"), RouteKind::Tar);
        assert_eq!(route_kind("/tmp/a.tgz/inner"), RouteKind::Tar);
        assert_eq!(route_kind("/tmp/a.tar.zst/x/y"), RouteKind::Tar);
    }

    #[test]
    fn lookalike_extensions_stay_local() {
        assert_eq!(route_kind("/tmp/file.zipx"), RouteKind::Local);
        assert_eq!(route_kind("/tmp/tarball.txt"), RouteKind::Local);
    }
}
