pub type ServiceResult<T> = anyhow::Result<T>;

pub mod fs_archive;
pub mod fs_config;
pub mod fs_delete;
pub mod fs_duplicates;
pub mod fs_listing;
pub mod fs_office;
pub mod fs_props;
pub mod fs_rename;
pub mod fs_size;
pub mod fs_transfer;
pub mod fs_vfs;
pub mod fs_watcher;
pub mod fs_zip;
pub mod remote;
