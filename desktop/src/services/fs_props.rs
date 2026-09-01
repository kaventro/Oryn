use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::services::ServiceResult;

#[derive(Debug, Clone)]
pub struct FsProps {
    pub size: u64,
    pub mode_oct: String,
    pub mode_string: String,
    pub is_dir: bool,
    pub mtime: String,
}

/// Shallow stat: for directories `size` is 0 — recursive content size is a
/// separate, potentially long scan (`fs_size::path_size`) that callers run
/// asynchronously instead of blocking every stat on a full tree walk.
pub fn stat_props(path: &Path) -> ServiceResult<FsProps> {
    let metadata = fs::metadata(path)?;
    let is_dir = metadata.is_dir();
    let size = if is_dir { 0 } else { metadata.len() };

    #[cfg(unix)]
    let (mode_oct, mode_string) = {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        (format!("{:03o}", mode & 0o777), format!("{:o}", mode))
    };

    #[cfg(not(unix))]
    let (mode_oct, mode_string) = ("644".to_string(), "644".to_string());

    let mtime = filetime_to_iso(&metadata);

    Ok(FsProps {
        size,
        mode_oct,
        mode_string,
        is_dir,
        mtime,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsPayload {
    pub path: String,
    pub size: u64,
    pub mode: String,
    pub mode_string: String,
    pub is_dir: bool,
    pub mtime: String,
}

#[derive(Serialize)]
pub struct StatPropsOut {
    pub ok: bool,
    pub props: PropsPayload,
}

pub fn stat_props_response(path: &str) -> ServiceResult<StatPropsOut> {
    let props = stat_props(Path::new(path))?;
    Ok(StatPropsOut {
        ok: true,
        props: PropsPayload {
            path: path.to_string(),
            size: props.size,
            mode: props.mode_oct,
            mode_string: props.mode_string,
            is_dir: props.is_dir,
            mtime: props.mtime,
        },
    })
}

fn filetime_to_iso(metadata: &fs::Metadata) -> String {
    use std::time::UNIX_EPOCH;
    let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
    let secs = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{stat_props, stat_props_response};
    use std::fs;

    #[test]
    fn stat_file_reports_size_and_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("f.bin");
        fs::write(&file, vec![0u8; 1234]).unwrap();

        let props = stat_props(&file).unwrap();
        assert_eq!(props.size, 1234);
        assert!(!props.is_dir);
        assert!(!props.mtime.is_empty());
    }

    #[test]
    fn stat_dir_is_shallow_and_instant() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a"), vec![0u8; 100]).unwrap();

        let props = stat_props(tmp.path()).unwrap();
        assert!(props.is_dir);
        assert_eq!(props.size, 0, "dir content size is computed separately");
    }

    #[test]
    fn response_keeps_wire_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("x.txt");
        fs::write(&file, b"hi").unwrap();

        let out = stat_props_response(file.to_str().unwrap()).unwrap();
        let json = serde_json::to_value(&out).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["props"]["size"], 2);
        assert_eq!(json["props"]["isDir"], false);
        assert!(json["props"]["modeString"].is_string());
    }

    #[test]
    fn stat_missing_path_is_an_error() {
        assert!(stat_props(std::path::Path::new("/definitely/not/here")).is_err());
    }
}
