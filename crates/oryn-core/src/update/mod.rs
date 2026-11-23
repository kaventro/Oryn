mod checksums;
mod install;
mod release;
mod target;
mod updater;

pub use checksums::{sha256_file, Checksums};
pub use install::{BinaryInstaller, BundleInstaller, Installer, TarballInstaller};
pub use release::{is_newer, parse_latest, parse_version, Asset, Release, LATEST_URL};
pub use target::{plan, Arch, Method, Os, Plan, Platform, Target};
pub use updater::{Fetch, Updater};
