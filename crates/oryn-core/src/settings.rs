use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub tray: usize,
    pub view: String,
    pub show_hidden: bool,
    pub confirm_delete: bool,
    pub use_trash: bool,
    pub dual: bool,
    pub sidebar: bool,
    pub folders_first: bool,
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tray: 0,
            view: "list".into(),
            show_hidden: true,
            confirm_delete: true,
            use_trash: true,
            dual: true,
            sidebar: true,
            folders_first: true,
            check_updates: true,
        }
    }
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        crate::services::fs_config::data_dir()
            .ok()
            .map(|d| d.join("settings.json"))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_fills_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/settings.json");
        assert_eq!(Settings::load_from(&path), Settings::default());
        let s = Settings {
            tray: 3,
            view: "grid".into(),
            show_hidden: false,
            ..Default::default()
        };
        s.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path), s);
        fs::write(&path, r#"{"tray": 2}"#).unwrap();
        let partial = Settings::load_from(&path);
        assert_eq!(partial.tray, 2);
        assert!(partial.use_trash);
        fs::write(&path, "not json").unwrap();
        assert_eq!(Settings::load_from(&path), Settings::default());
        assert!(Settings::path().is_some());
    }
}
