mod router;
pub mod types;
mod providers {
    pub mod local;
    pub mod tar;
    pub mod zip;
}

pub use router::{is_archive_file, is_virtual, VfsRouter};
pub use types::{FileItem, VirtualFileSystem};
