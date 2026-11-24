use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Checksums {
    by_name: HashMap<String, String>,
}

impl Checksums {
    pub fn parse(text: &str) -> Self {
        let by_name = text
            .lines()
            .filter_map(|line| {
                let (hash, name) = line.trim().split_once(char::is_whitespace)?;
                let name = name.trim_start().trim_start_matches('*');
                let valid = hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit());
                (valid && !name.is_empty()).then(|| (name.to_string(), hash.to_ascii_lowercase()))
            })
            .collect();
        Self { by_name }
    }

    pub fn expected(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).map(String::as_str)
    }

    pub fn verify(&self, name: &str, path: &Path) -> Result<()> {
        let Some(expected) = self.expected(name) else {
            bail!("The release has no checksum for {name}.");
        };
        if sha256_file(path)? != expected {
            bail!("The download of {name} is damaged. Its checksum doesn't match.");
        }
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn parses_sha256sum_and_shasum_output() {
        let text = format!(
            "{EMPTY}  Oryn_0.0.11_x64_portable.exe\n{}  *oryn_0.0.11_amd64.deb\nnot a line\n",
            EMPTY.to_uppercase()
        );
        let sums = Checksums::parse(&text);
        assert_eq!(sums.expected("Oryn_0.0.11_x64_portable.exe"), Some(EMPTY));
        assert_eq!(sums.expected("oryn_0.0.11_amd64.deb"), Some(EMPTY));
        assert_eq!(sums.expected("not"), None);
    }

    #[test]
    fn verify_accepts_a_match_and_rejects_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a");
        std::fs::write(&file, b"").unwrap();
        let sums = Checksums::parse(&format!("{EMPTY}  a\n{}  b\n", "0".repeat(64)));
        assert!(sums.verify("a", &file).is_ok());
        assert!(sums.verify("b", &file).is_err());
        assert!(sums.verify("c", &file).is_err());
    }
}
