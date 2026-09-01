use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use super::shared::{binary_differ, buffer_looks_binary, filetime_ms, MAX_JS_DIFF};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareFilesIn {
    pub left_path: String,
    pub right_path: String,
}

pub fn compare_files(input: CompareFilesIn) -> Result<Value, String> {
    let l = Path::new(&input.left_path);
    let r = Path::new(&input.right_path);
    let ls = fs::metadata(l).map_err(|e| e.to_string())?;
    let rs = fs::metadata(r).map_err(|e| e.to_string())?;
    if !ls.is_file() || !rs.is_file() {
        return Err("Both paths must be regular files.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if ls.dev() == rs.dev() && ls.ino() == rs.ino() && ls.ino() != 0 {
            return Ok(json!({"ok": true, "diff": "", "same": true, "reason": "same-inode"}));
        }
    }
    if ls.len() == rs.len() && (filetime_ms(&ls) - filetime_ms(&rs)).abs() < 2000.0 {
        return Ok(json!({"ok": true, "diff": "", "same": true, "reason": "metadata"}));
    }
    let maxb = ls.len().max(rs.len());
    if maxb <= MAX_JS_DIFF {
        let buf_l = fs::read(l).map_err(|e| e.to_string())?;
        let buf_r = fs::read(r).map_err(|e| e.to_string())?;
        if buf_l == buf_r {
            return Ok(json!({"ok": true, "diff": "", "same": true, "reason": "byte-identical"}));
        }
        if buffer_looks_binary(&buf_l) || buffer_looks_binary(&buf_r) {
            return Ok(json!({
                "ok": true,
                "diff": format!("Binary files differ ({} vs {} bytes).\n", ls.len(), rs.len()),
                "same": false,
                "engine": "binary"
            }));
        }
        let a = String::from_utf8_lossy(&buf_l);
        let b = String::from_utf8_lossy(&buf_r);
        let patch = diffy::create_patch(a.as_ref(), b.as_ref());
        let mut out = format!("--- {}\n+++ {}\n", input.left_path, input.right_path);
        out.push_str(&format!("{}", patch));
        return Ok(json!({"ok": true, "diff": out, "same": false, "engine": "diffy"}));
    }
    if ls.len() != rs.len() {
        return Ok(json!({
            "ok": true,
            "diff": format!("Files differ in size ({} vs {} bytes).\n", ls.len(), rs.len()),
            "same": false,
            "engine": "size-only"
        }));
    }
    match binary_differ(l, r) {
        Ok(false) => Ok(json!({"ok": true, "diff": "", "same": true, "reason": "stream-compare"})),
        _ => Ok(json!({
            "ok": true,
            "diff": "Large files differ; open externally for full diff.\n",
            "same": false,
            "engine": "fallback"
        })),
    }
}
