use anyhow::{anyhow, Context, Result};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub struct SafeRoot {
    dir: Dir,
    root: PathBuf,
}

impl SafeRoot {
    pub fn open(root: &Path) -> Result<Self> {
        let dir = Dir::open_ambient_dir(root, ambient_authority())
            .with_context(|| format!("Failed to open sandbox root {}", root.display()))?;
        Ok(Self {
            dir,
            root: root.to_path_buf(),
        })
    }

    pub fn create_dir_all(&self, rel: &Path) -> Result<PathBuf> {
        let rel = sanitize_rel(rel)?;
        if !rel.as_os_str().is_empty() {
            self.dir.create_dir_all(&rel).with_context(|| {
                format!("Failed to create directory {} in sandbox", rel.display())
            })?;
        }
        Ok(self.root.join(&rel))
    }

    pub fn write_file(
        &self,
        rel: &Path,
        reader: &mut impl Read,
        max_bytes: u64,
        unix_mode: Option<u32>,
    ) -> Result<u64> {
        let safe = sanitize_rel(rel)?;
        let mut out = self.create_file(rel, unix_mode)?;

        let result = (|| -> Result<u64> {
            let mut limited = (&mut *reader).take(max_bytes + 1);
            let written = std::io::copy(&mut limited, &mut out)?;
            if written > max_bytes {
                return Err(anyhow!(
                    "Archive entry exceeds maximum extract size ({} bytes)",
                    max_bytes
                ));
            }
            Ok(written)
        })();
        drop(out);

        if result.is_err() {
            let _ = self.dir.remove_file(&safe);
        }
        result
    }

    pub fn create_file(&self, rel: &Path, unix_mode: Option<u32>) -> Result<File> {
        let rel = sanitize_rel(rel)?;
        if rel.as_os_str().is_empty() {
            return Err(anyhow!("Refusing to write an entry with an empty path"));
        }
        if let Some(parent) = rel.parent() {
            if !parent.as_os_str().is_empty() {
                self.dir.create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent of {} in sandbox", rel.display())
                })?;
            }
        }
        let _ = self.dir.remove_file(&rel);

        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        let file = self
            .dir
            .open_with(&rel, &opts)
            .with_context(|| format!("Failed to create {} in sandbox", rel.display()))?;

        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::fchmod(file.as_raw_fd(), mode as libc::mode_t);
            }
        }
        #[cfg(not(unix))]
        let _ = unix_mode;

        Ok(file)
    }

    pub fn resolved_path(&self, rel: &Path) -> Result<PathBuf> {
        Ok(self.root.join(sanitize_rel(rel)?))
    }
}

fn sanitize_rel(rel: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!(
                    "Absolute entry path is not allowed: {}",
                    rel.display()
                ));
            }
            Component::ParentDir => {
                return Err(anyhow!(
                    "Entry path escapes the root via '..': {}",
                    rel.display()
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn root() -> (tempfile::TempDir, SafeRoot) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let safe = SafeRoot::open(&root).unwrap();
        (tmp, safe)
    }

    struct FailAfter {
        ok_bytes: usize,
        sent: usize,
    }
    impl Read for FailAfter {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.sent >= self.ok_bytes {
                return Err(std::io::Error::other("boom"));
            }
            let n = buf.len().min(self.ok_bytes - self.sent);
            for b in buf.iter_mut().take(n) {
                *b = 1;
            }
            self.sent += n;
            Ok(n)
        }
    }

    #[test]
    fn writes_nested_within_limit() {
        let (tmp, safe) = root();
        let n = safe
            .write_file(Path::new("a/b/c.txt"), &mut &[7u8; 100][..], 1000, None)
            .unwrap();
        assert_eq!(n, 100);
        assert_eq!(
            fs::read(tmp.path().join("root/a/b/c.txt")).unwrap(),
            [7u8; 100]
        );
    }

    #[test]
    fn rejects_oversize_and_cleans_up() {
        let (tmp, safe) = root();
        assert!(safe
            .write_file(Path::new("big.bin"), &mut &[7u8; 100][..], 10, None)
            .is_err());
        assert!(!tmp.path().join("root/big.bin").exists());
    }

    #[test]
    fn removes_partial_on_io_error() {
        let (tmp, safe) = root();
        let mut reader = FailAfter {
            ok_bytes: 32,
            sent: 0,
        };
        assert!(safe
            .write_file(Path::new("part.bin"), &mut reader, 1_000_000, None)
            .is_err());
        assert!(!tmp.path().join("root/part.bin").exists());
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let (tmp, safe) = root();
        assert!(safe
            .write_file(Path::new("../escape.txt"), &mut &[1u8; 4][..], 1000, None)
            .is_err());
        assert!(!tmp.path().join("escape.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_follow_symlinked_parent_out_of_root() {
        let (tmp, safe) = root();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink("../outside", tmp.path().join("root/link")).unwrap();

        let res = safe.write_file(Path::new("link/evil.txt"), &mut &[9u8; 4][..], 1000, None);
        assert!(res.is_err());
        assert!(!outside.join("evil.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn applies_unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        let (tmp, safe) = root();
        safe.write_file(
            Path::new("script.sh"),
            &mut &b"#!/bin/sh\n"[..],
            1000,
            Some(0o755),
        )
        .unwrap();
        let mode = fs::metadata(tmp.path().join("root/script.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}
