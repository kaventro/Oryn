use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const COLORS: [&str; 7] = ["Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Gray"];

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tags {
    #[serde(flatten)]
    map: BTreeMap<String, Vec<String>>,
}

fn key(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

impl Tags {
    pub fn path() -> Option<PathBuf> {
        crate::services::fs_config::data_dir()
            .ok()
            .map(|d| d.join("tags.json"))
    }

    pub fn load_from(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)
    }

    pub fn load() -> Self {
        Self::path()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            let _ = self.save_to(&p);
        }
    }

    pub fn of(&self, p: &Path) -> &[String] {
        self.map.get(&key(p)).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn has(&self, p: &Path, color: &str) -> bool {
        self.of(p).iter().any(|c| c == color)
    }

    pub fn toggle(&mut self, paths: &[PathBuf], color: &str) -> bool {
        let on = !paths.iter().all(|p| self.has(p, color));
        for p in paths {
            let k = key(p);
            let list = self.map.entry(k.clone()).or_default();
            list.retain(|c| c != color);
            if on {
                list.push(color.to_string());
                list.sort_by_key(|c| COLORS.iter().position(|x| x == c).unwrap_or(99));
            }
            if list.is_empty() {
                self.map.remove(&k);
            }
        }
        on
    }

    pub fn tagged(&self, color: &str) -> Vec<PathBuf> {
        self.map
            .iter()
            .filter(|(_, v)| v.iter().any(|c| c == color))
            .map(|(k, _)| PathBuf::from(k))
            .filter(|p| p.exists())
            .collect()
    }

    pub fn moved(&mut self, from: &Path, to: &Path) {
        let prefix = key(from);
        let moved: Vec<(String, Vec<String>)> = self
            .map
            .iter()
            .filter(|(k, _)| {
                k.as_str() == prefix
                    || k.strip_prefix(&prefix)
                        .is_some_and(|r| r.starts_with(['/', '\\']))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in moved {
            self.map.remove(&k);
            let rest = &k[prefix.len()..];
            self.map.insert(format!("{}{}", key(to), rest), v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_lists_and_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();
        let mut t = Tags::default();
        assert!(t.toggle(std::slice::from_ref(&a), "Blue"));
        assert!(t.toggle(&[a.clone(), b.clone()], "Red"));
        assert_eq!(t.of(&a), ["Red", "Blue"]);
        assert!(!t.toggle(std::slice::from_ref(&a), "Blue"));
        assert_eq!(t.tagged("Red").len(), 2);
        assert!(!t.toggle(&[a.clone(), b.clone()], "Red"));
        assert!(t.of(&b).is_empty());
        t.toggle(std::slice::from_ref(&a), "Green");
        let file = tmp.path().join("cfg/tags.json");
        t.save_to(&file).unwrap();
        assert_eq!(Tags::load_from(&file), t);
        t.moved(&a, &tmp.path().join("c"));
        assert!(t.of(&a).is_empty());
        assert_eq!(t.of(&tmp.path().join("c")), ["Green"]);
    }
}
