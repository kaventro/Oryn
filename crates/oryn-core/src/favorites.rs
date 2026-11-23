use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Favorites {
    pub paths: Vec<PathBuf>,
}

fn same(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy();
        let t = s.trim_end_matches(['/', '\\']);
        if t.is_empty() {
            s.into_owned()
        } else {
            t.to_string()
        }
    };
    if cfg!(any(windows, target_os = "macos")) {
        norm(a).eq_ignore_ascii_case(&norm(b))
    } else {
        norm(a) == norm(b)
    }
}

impl Favorites {
    pub fn defaults(home: &Path) -> Self {
        let mut paths = vec![home.to_path_buf()];
        for sub in ["Desktop", "Documents", "Downloads"] {
            let p = home.join(sub);
            if p.is_dir() {
                paths.push(p);
            }
        }
        Self { paths }
    }

    pub fn path() -> Option<PathBuf> {
        crate::services::fs_config::data_dir()
            .ok()
            .map(|d| d.join("favorites.json"))
    }

    pub fn load_from(path: &Path, home: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| Self::defaults(home))
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

    pub fn load(home: &Path) -> Self {
        Self::path()
            .map(|p| Self::load_from(&p, home))
            .unwrap_or_else(|| Self::defaults(home))
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            let _ = self.save_to(&p);
        }
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| same(p, path))
    }

    pub fn add(&mut self, path: &Path) -> bool {
        if self.contains(path) {
            return false;
        }
        self.paths.push(path.to_path_buf());
        true
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.paths.len();
        self.paths.retain(|p| !same(p, path));
        self.paths.len() != before
    }

    pub fn label(path: &Path) -> String {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_list_home_and_existing_folders() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("Documents")).unwrap();
        let favs = Favorites::defaults(tmp.path());
        assert_eq!(
            favs.paths,
            vec![tmp.path().to_path_buf(), tmp.path().join("Documents")]
        );
    }

    #[test]
    fn add_remove_and_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("nested/favorites.json");
        let mut favs = Favorites::load_from(&file, tmp.path());
        let work = tmp.path().join("work");
        assert!(favs.add(&work));
        assert!(!favs.add(&tmp.path().join("work/")));
        favs.save_to(&file).unwrap();
        let loaded = Favorites::load_from(&file, Path::new("/elsewhere"));
        assert!(loaded.contains(&work));
        let mut loaded = loaded;
        assert!(loaded.remove(&work));
        assert!(!loaded.contains(&work));
        assert!(!loaded.remove(&work));
        assert_eq!(Favorites::label(&work), "work");
    }

    #[test]
    fn an_emptied_list_stays_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("favorites.json");
        Favorites::default().save_to(&file).unwrap();
        assert!(Favorites::load_from(&file, tmp.path()).paths.is_empty());
    }
}
