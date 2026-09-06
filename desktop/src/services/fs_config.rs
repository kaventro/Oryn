use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::services::ServiceResult;

pub struct DeleteOptions {
    pub use_trash: bool,
    pub log_path: Option<PathBuf>,
}

pub fn data_dir() -> ServiceResult<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("no data dir"))?
        .join("Oryn");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> ServiceResult<PathBuf> {
    Ok(data_dir()?.join("config.json"))
}

pub fn deletion_log_path() -> ServiceResult<PathBuf> {
    Ok(data_dir()?.join("deleted_paths.log"))
}

pub fn load_config() -> ServiceResult<Value> {
    let defaults = json!({"useTrash": true, "deletionLog": true});
    let path = config_path()?;
    if !path.exists() {
        return Ok(defaults);
    }

    let source = fs::read_to_string(&path)?;
    let mut value: Value = match serde_json::from_str(&source) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("config.json is invalid ({e}); falling back to defaults");
            defaults.clone()
        }
    };

    if let (Value::Object(map), Value::Object(defaults_map)) = (&mut value, &defaults) {
        for (key, default_value) in defaults_map {
            map.entry(key.clone())
                .or_insert_with(|| default_value.clone());
        }
    }

    Ok(value)
}

pub fn resolve_delete_options(use_trash_override: Option<bool>) -> ServiceResult<DeleteOptions> {
    let cfg = load_config()?;
    let trash_default = cfg
        .get("useTrash")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let log_on = cfg
        .get("deletionLog")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let use_trash = use_trash_override.unwrap_or(trash_default);
    let log_path = if log_on {
        Some(deletion_log_path()?)
    } else {
        None
    };

    Ok(DeleteOptions {
        use_trash,
        log_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paths() {
        assert!(data_dir().is_ok());
        let cfg_path = config_path().unwrap();
        assert!(cfg_path.ends_with("config.json"));
        let del_log = deletion_log_path().unwrap();
        assert!(del_log.ends_with("deleted_paths.log"));
    }

    #[test]
    fn test_load_config_and_resolve_delete_options() {
        let cfg = load_config().unwrap();
        assert!(cfg.is_object());

        let opts_default = resolve_delete_options(None).unwrap();
        // Default use_trash is true unless overridden by existing config
        assert!(opts_default.log_path.is_some());

        let opts_override_false = resolve_delete_options(Some(false)).unwrap();
        assert!(!opts_override_false.use_trash);

        let opts_override_true = resolve_delete_options(Some(true)).unwrap();
        assert!(opts_override_true.use_trash);
    }
}
