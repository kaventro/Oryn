use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use sha2::{Digest, Sha256, Sha512};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sums {
    pub sha256: String,
    pub sha512: String,
}

pub fn hash_file(
    path: &Path,
    done: &AtomicU64,
    cancel: &AtomicBool,
) -> std::io::Result<Option<Sums>> {
    let mut file = File::open(path)?;
    let mut a = Sha256::new();
    let mut b = Sha512::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        a.update(&buf[..n]);
        b.update(&buf[..n]);
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(Some(Sums {
        sha256: hex::encode(a.finalize()),
        sha512: hex::encode(b.finalize()),
    }))
}

pub fn matches(sums: &Sums, expected: &str) -> Option<&'static str> {
    let e = expected.trim().to_ascii_lowercase();
    if e.is_empty() {
        None
    } else if e == sums.sha256 {
        Some("SHA-256")
    } else if e == sums.sha512 {
        Some("SHA-512")
    } else {
        Some("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_known_text() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("abc");
        std::fs::write(&p, "abc").unwrap();
        let done = AtomicU64::new(0);
        let s = hash_file(&p, &done, &AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert_eq!(
            s.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(s.sha512.starts_with("ddaf35a193617aba"));
        assert_eq!(done.load(Ordering::Relaxed), 3);
        assert_eq!(
            matches(
                &s,
                " BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD "
            ),
            Some("SHA-256")
        );
        assert_eq!(matches(&s, "nope"), Some(""));
        assert_eq!(matches(&s, ""), None);
        assert!(hash_file(&p, &done, &AtomicBool::new(true))
            .unwrap()
            .is_none());
    }
}
