use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::checksums::Checksums;
use super::release::Release;
use super::target::Target;

pub trait Fetch: Send + Sync {
    fn text(&self, url: &str) -> Result<String>;
    fn save(&self, url: &str, dest: &Path) -> Result<()>;
}

pub struct Updater<F: Fetch> {
    fetch: F,
    work: PathBuf,
}

impl<F: Fetch> Updater<F> {
    pub fn new(fetch: F, work: PathBuf) -> Self {
        Self { fetch, work }
    }

    pub fn install(&self, release: &Release, target: &Target) -> Result<PathBuf> {
        let _ = fs::remove_dir_all(&self.work);
        fs::create_dir_all(&self.work)?;
        let result = self.run(release, target);
        let _ = fs::remove_dir_all(&self.work);
        result
    }

    fn run(&self, release: &Release, target: &Target) -> Result<PathBuf> {
        let asset = release
            .asset(&target.asset)
            .context("This release has no download for your system yet.")?;
        let sums = release
            .asset(&target.checksums)
            .context("This release has no checksums, so Oryn can't check the download.")?;
        let checksums = Checksums::parse(
            &self
                .fetch
                .text(&sums.url)
                .context("Oryn couldn't download the checksums.")?,
        );
        let file = self.work.join(&asset.name);
        self.fetch
            .save(&asset.url, &file)
            .context("Oryn couldn't download the update.")?;
        checksums.verify(&asset.name, &file)?;
        target.installer().install(&file, &self.work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::release::Asset;
    use crate::update::target::Method;
    use std::collections::HashMap;

    struct Canned(HashMap<&'static str, Vec<u8>>);

    impl Fetch for Canned {
        fn text(&self, url: &str) -> Result<String> {
            Ok(String::from_utf8(self.0.get(url).context("404")?.clone())?)
        }

        fn save(&self, url: &str, dest: &Path) -> Result<()> {
            fs::write(dest, self.0.get(url).context("404")?)?;
            Ok(())
        }
    }

    fn release() -> Release {
        let asset = |name: &str, url: &str| Asset {
            name: name.into(),
            url: url.into(),
        };
        Release {
            version: "1.0.0".into(),
            url: "page".into(),
            assets: vec![
                asset("Oryn_1.0.0_x64_portable.exe", "exe"),
                asset("SHA256SUMS-windows.txt", "sums"),
            ],
        }
    }

    fn target(exe: &Path) -> Target {
        Target {
            asset: "Oryn_1.0.0_x64_portable.exe".into(),
            checksums: "SHA256SUMS-windows.txt".into(),
            method: Method::Binary {
                exe: exe.to_path_buf(),
            },
        }
    }

    fn sums_for(body: &[u8]) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, body).unwrap();
        let hash = crate::update::sha256_file(&file).unwrap();
        format!("{hash}  Oryn_1.0.0_x64_portable.exe\n").into_bytes()
    }

    #[test]
    fn installs_a_verified_download_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Oryn.exe");
        fs::write(&exe, b"old").unwrap();
        let work = dir.path().join("work");
        let fetch = Canned(HashMap::from([
            ("exe", b"new".to_vec()),
            ("sums", sums_for(b"new")),
        ]));
        let back = Updater::new(fetch, work.clone())
            .install(&release(), &target(&exe))
            .unwrap();
        assert_eq!(back, exe);
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert!(!work.exists());
    }

    #[test]
    fn refuses_a_download_that_fails_its_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Oryn.exe");
        fs::write(&exe, b"old").unwrap();
        let fetch = Canned(HashMap::from([
            ("exe", b"tampered".to_vec()),
            ("sums", sums_for(b"new")),
        ]));
        let err = Updater::new(fetch, dir.path().join("work"))
            .install(&release(), &target(&exe))
            .unwrap_err();
        assert!(err.to_string().contains("damaged"));
        assert_eq!(fs::read(&exe).unwrap(), b"old");
    }

    #[test]
    fn refuses_a_release_without_checksums() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Oryn.exe");
        fs::write(&exe, b"old").unwrap();
        let mut release = release();
        release.assets.pop();
        let fetch = Canned(HashMap::from([("exe", b"new".to_vec())]));
        assert!(Updater::new(fetch, dir.path().join("work"))
            .install(&release, &target(&exe))
            .is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"old");
    }
}
