pub mod format;
pub mod icons;

pub mod services {
    pub type ServiceResult<T> = anyhow::Result<T>;
}

#[rustfmt::skip]
#[path = "../../../desktop/src/services/fs_listing.rs"]
pub mod fs_listing;

pub use fs_listing::{ListItem, list_dir};
