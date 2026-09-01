use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathArg {
    pub path: String,
}

#[tauri::command]
pub fn path_join(a: String, b: String) -> Result<String, String> {
    Ok(Path::new(&a).join(&b).to_string_lossy().to_string())
}

#[tauri::command]
pub fn path_dirname(input: PathArg) -> Result<String, String> {
    let raw = input.path.trim();
    let p = Path::new(raw);
    Ok(p.parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn path_basename(input: PathArg) -> Result<String, String> {
    Ok(Path::new(&input.path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default())
}

fn lexical_normalize(path: &str) -> String {
    let mut out = PathBuf::new();
    let input = Path::new(path);

    for comp in input.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(comp.as_os_str());
                }
            }
            other => out.push(other.as_os_str()),
        }
    }

    if out.as_os_str().is_empty() {
        ".".to_string()
    } else {
        out.to_string_lossy().to_string()
    }
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
    }
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        return dirs::home_dir()
            .map(|p| p.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
    }
    path.to_string()
}

pub fn clean_verbatim_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

#[tauri::command]
pub fn path_normalize(input: PathArg) -> Result<String, String> {
    let raw = input.path.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }

    let expanded = expand_home(raw);

    match std::fs::canonicalize(&expanded) {
        Ok(p) => {
            let s = p.to_string_lossy();
            Ok(clean_verbatim_path(&s))
        }
        Err(_) => Ok(clean_verbatim_path(&lexical_normalize(&expanded))),
    }
}

#[tauri::command]
pub fn app_get_home() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "no home dir".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_windows_verbatim_prefix() {
        assert_eq!(clean_verbatim_path(r"\\?\C:\Users"), r"C:\Users");
        assert_eq!(
            clean_verbatim_path(r"\\?\UNC\server\share"),
            r"\\server\share"
        );
        assert_eq!(clean_verbatim_path("/Users/test"), "/Users/test");
    }

    #[test]
    fn expands_home_prefix() {
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy().to_string();
            assert_eq!(expand_home("~"), home_str);
            assert_eq!(
                expand_home("~/test"),
                home.join("test").to_string_lossy().to_string()
            );
            assert_eq!(
                expand_home("~\\test"),
                home.join("test").to_string_lossy().to_string()
            );
        }
    }
}
