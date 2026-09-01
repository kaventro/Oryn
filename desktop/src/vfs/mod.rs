mod router;
pub mod types;
mod providers {
    pub mod local;
    pub mod tar;
    pub mod zip;
}

pub use router::VfsRouter;
pub use types::{FileItem, VirtualFileSystem};
