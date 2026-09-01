use serde_json::{json, Value};
use std::fs;
use zip::ZipArchive;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipExtractIn {
    pub zip_path: String,
    pub entry_name: String,
}

pub fn zip_extract(input: ZipExtractIn) -> Result<Value, String> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("ow-zip-{}-{}", std::process::id(), nonce));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o700));
    }
    let zf = fs::File::open(&input.zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(zf).map_err(|e| e.to_string())?;
    let mut z = archive
        .by_name(&input.entry_name)
        .map_err(|e| e.to_string())?;

    const MAX_ZIP_ENTRY_EXTRACT_BYTES: u64 = 500 * 1024 * 1024;
    let rel = std::path::Path::new(&input.entry_name);
    let root = crate::fs_safe::SafeRoot::open(&tmp).map_err(|e| e.to_string())?;
    let out_path = root.resolved_path(rel).map_err(|e| e.to_string())?;
    root.write_file(rel, &mut z, MAX_ZIP_ENTRY_EXTRACT_BYTES, None)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "ok": true,
        "path": out_path.to_string_lossy().to_string(),
        "tmpDir": tmp.to_string_lossy().to_string()
    }))
}
