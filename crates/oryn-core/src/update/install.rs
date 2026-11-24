use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;

pub trait Installer: Send {
    fn install(&self, download: &Path, work: &Path) -> Result<PathBuf>;
}

pub struct BinaryInstaller {
    exe: PathBuf,
}

impl BinaryInstaller {
    pub fn new(exe: PathBuf) -> Self {
        Self { exe }
    }
}

impl Installer for BinaryInstaller {
    fn install(&self, download: &Path, _work: &Path) -> Result<PathBuf> {
        replace_file(download, &self.exe)?;
        Ok(self.exe.clone())
    }
}

pub struct TarballInstaller {
    exe: PathBuf,
    member: String,
}

impl TarballInstaller {
    pub fn new(exe: PathBuf, member: String) -> Self {
        Self { exe, member }
    }

    fn extract(&self, download: &Path, work: &Path) -> Result<PathBuf> {
        let mut archive = tar::Archive::new(GzDecoder::new(File::open(download)?));
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.as_ref() == Path::new(&self.member) {
                let out = work.join("oryn.new");
                entry.unpack(&out)?;
                return Ok(out);
            }
        }
        bail!("The download doesn't contain {}.", self.member)
    }
}

impl Installer for TarballInstaller {
    fn install(&self, download: &Path, work: &Path) -> Result<PathBuf> {
        let binary = self.extract(download, work)?;
        replace_file(&binary, &self.exe)?;
        Ok(self.exe.clone())
    }
}

pub struct BundleInstaller {
    app: PathBuf,
}

impl BundleInstaller {
    pub fn new(app: PathBuf) -> Self {
        Self { app }
    }

    fn sibling(&self, suffix: &str) -> Result<PathBuf> {
        let name = self
            .app
            .file_name()
            .context("Oryn.app has no name.")?
            .to_string_lossy();
        Ok(self.app.with_file_name(format!(".{name}.{suffix}")))
    }

    fn unpack(&self, download: &Path) -> Result<PathBuf> {
        let stage = self.sibling("update")?;
        remove_if_present(&stage)?;
        fs::create_dir_all(&stage).with_context(|| {
            format!(
                "Oryn can't write to {}. Update it from the release page instead.",
                stage.parent().unwrap_or(&stage).display()
            )
        })?;
        unzip(download, &stage)?;
        let app = fs::read_dir(&stage)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .find(|p| p.extension().is_some_and(|x| x == "app"))
            .context("The download doesn't contain Oryn.app.")?;
        if !app.join("Contents").join("MacOS").is_dir() {
            bail!("The downloaded Oryn.app is incomplete.");
        }
        verify_bundle(&app)?;
        Ok(app)
    }

    fn swap(&self, fresh: &Path) -> Result<()> {
        let old = self.sibling("old")?;
        remove_if_present(&old)?;
        fs::rename(&self.app, &old).context("Oryn couldn't move the current version aside.")?;
        if let Err(err) = fs::rename(fresh, &self.app) {
            fs::rename(&old, &self.app)?;
            return Err(err).context("Oryn couldn't put the new version in place.");
        }
        let _ = fs::remove_dir_all(&old);
        Ok(())
    }
}

impl Installer for BundleInstaller {
    fn install(&self, download: &Path, _work: &Path) -> Result<PathBuf> {
        let result = self.unpack(download).and_then(|fresh| self.swap(&fresh));
        if let Ok(stage) = self.sibling("update") {
            let _ = fs::remove_dir_all(stage);
        }
        result.map(|()| self.app.clone())
    }
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

#[cfg(target_os = "macos")]
fn unzip(zip: &Path, into: &Path) -> Result<()> {
    run("/usr/bin/ditto", &["-x", "-k"], &[zip, into])
}

#[cfg(not(target_os = "macos"))]
fn unzip(zip: &Path, into: &Path) -> Result<()> {
    zip::ZipArchive::new(File::open(zip)?)?.extract(into)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_bundle(app: &Path) -> Result<()> {
    run(
        "/usr/bin/codesign",
        &["--verify", "--deep", "--strict"],
        &[app],
    )
    .context("The downloaded Oryn.app failed its code signature check.")?;
    let _ = run("/usr/bin/xattr", &["-dr", "com.apple.quarantine"], &[app]);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn verify_bundle(_app: &Path) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn run(program: &str, flags: &[&str], paths: &[&Path]) -> Result<()> {
    let status = std::process::Command::new(program)
        .args(flags)
        .args(paths)
        .status()?;
    if !status.success() {
        bail!("{program} failed with {status}");
    }
    Ok(())
}

fn replace_file(fresh: &Path, dest: &Path) -> Result<()> {
    make_executable(fresh)?;
    if is_running(dest) {
        return self_replace::self_replace(fresh)
            .context("Oryn couldn't replace its own program file.");
    }
    let name = dest.file_name().context("The program has no name.")?;
    let staged = dest.with_file_name(format!(".{}.update", name.to_string_lossy()));
    fs::copy(fresh, &staged).with_context(|| format!("Oryn can't write to {}.", dest.display()))?;
    fs::rename(&staged, dest).inspect_err(|_| {
        let _ = fs::remove_file(&staged);
    })?;
    Ok(())
}

fn is_running(path: &Path) -> bool {
    let running = std::env::current_exe().and_then(fs::canonicalize);
    matches!((running, fs::canonicalize(path)), (Ok(a), Ok(b)) if a == b)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tarball(path: &Path, member: &str, body: &[u8]) {
        let gz =
            flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, member, body).unwrap();
        builder.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn binary_installer_replaces_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("oryn");
        let fresh = dir.path().join("download");
        fs::write(&exe, b"old").unwrap();
        fs::write(&fresh, b"new").unwrap();
        let back = BinaryInstaller::new(exe.clone())
            .install(&fresh, dir.path())
            .unwrap();
        assert_eq!(back, exe);
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert!(!dir.path().join(".oryn.update").exists());
    }

    #[test]
    fn tarball_installer_takes_the_binary_from_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("oryn");
        fs::write(&exe, b"old").unwrap();
        let download = dir.path().join("Oryn.tar.gz");
        tarball(&download, "Oryn_1.0.0_linux_x86_64/oryn", b"new");
        let installer = TarballInstaller::new(exe.clone(), "Oryn_1.0.0_linux_x86_64/oryn".into());
        installer.install(&download, dir.path()).unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&exe).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn tarball_installer_rejects_an_archive_without_oryn() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("oryn");
        fs::write(&exe, b"old").unwrap();
        let download = dir.path().join("Oryn.tar.gz");
        tarball(&download, "something/else", b"new");
        let installer = TarballInstaller::new(exe.clone(), "Oryn/oryn".into());
        assert!(installer.install(&download, dir.path()).is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"old");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn bundle_installer_swaps_the_whole_app() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Oryn.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/oryn"), b"old").unwrap();
        fs::write(app.join("Contents/stale"), b"gone").unwrap();
        let download = dir.path().join("Oryn.app.zip");
        let mut zip = zip::ZipWriter::new(File::create(&download).unwrap());
        let opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("Oryn.app/Contents/MacOS/oryn", opts)
            .unwrap();
        zip.write_all(b"new").unwrap();
        zip.start_file("Oryn.app/Contents/Info.plist", opts)
            .unwrap();
        zip.write_all(b"plist").unwrap();
        zip.finish().unwrap();
        let back = BundleInstaller::new(app.clone())
            .install(&download, dir.path())
            .unwrap();
        assert_eq!(back, app);
        assert_eq!(fs::read(app.join("Contents/MacOS/oryn")).unwrap(), b"new");
        assert!(app.join("Contents/Info.plist").exists());
        assert!(!app.join("Contents/stale").exists());
        assert!(!dir.path().join(".Oryn.app.old").exists());
        assert!(!dir.path().join(".Oryn.app.update").exists());
    }

    #[test]
    fn bundle_installer_keeps_the_app_when_the_download_is_wrong() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Oryn.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/oryn"), b"old").unwrap();
        let download = dir.path().join("Oryn.app.zip");
        fs::write(&download, b"not a zip").unwrap();
        assert!(BundleInstaller::new(app.clone())
            .install(&download, dir.path())
            .is_err());
        assert_eq!(fs::read(app.join("Contents/MacOS/oryn")).unwrap(), b"old");
        assert!(!dir.path().join(".Oryn.app.update").exists());
    }

    #[test]
    fn binary_installer_reports_a_destination_it_cannot_replace() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("oryn");
        fs::create_dir_all(exe.join("inside")).unwrap();
        let fresh = dir.path().join("download");
        fs::write(&fresh, b"new").unwrap();
        assert!(BinaryInstaller::new(exe.clone())
            .install(&fresh, dir.path())
            .is_err());
        assert!(exe.join("inside").is_dir());
        assert!(!dir.path().join(".oryn.update").exists());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn bundle_installer_rejects_an_incomplete_app() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Oryn.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        let download = dir.path().join("Oryn.app.zip");
        let mut zip = zip::ZipWriter::new(File::create(&download).unwrap());
        zip.start_file(
            "Oryn.app/Info.plist",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut zip, b"plist").unwrap();
        zip.finish().unwrap();
        let err = BundleInstaller::new(app.clone())
            .install(&download, dir.path())
            .unwrap_err();
        assert!(err.to_string().contains("incomplete"));
        assert!(app.join("Contents/MacOS").is_dir());
    }
}
