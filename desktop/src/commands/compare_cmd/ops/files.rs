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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compare_files_errors() {
        let tmp = tempdir().unwrap();
        let f1 = tmp.path().join("f1.txt");
        let d1 = tmp.path().join("dir");
        std::fs::create_dir(&d1).unwrap();
        std::fs::write(&f1, b"hello").unwrap();

        // Nonexistent
        assert!(compare_files(CompareFilesIn {
            left_path: "missing.txt".into(),
            right_path: f1.to_str().unwrap().into(),
        }).is_err());

        // Directory
        assert!(compare_files(CompareFilesIn {
            left_path: d1.to_str().unwrap().into(),
            right_path: f1.to_str().unwrap().into(),
        }).is_err());
    }

    #[test]
    fn test_compare_files_identical_and_diffy() {
        let tmp = tempdir().unwrap();
        let f1 = tmp.path().join("left.txt");
        let f2 = tmp.path().join("right.txt");
        let f3 = tmp.path().join("different.txt");

        std::fs::write(&f1, "line 1\nline 2\n").unwrap();
        std::fs::write(&f2, "line 1\nline 2\n").unwrap();
        std::fs::write(&f3, "line 1\nline changed\n").unwrap();

        // Identical
        let same_res = compare_files(CompareFilesIn {
            left_path: f1.to_str().unwrap().into(),
            right_path: f2.to_str().unwrap().into(),
        }).unwrap();
        assert_eq!(same_res["ok"], true);
        assert_eq!(same_res["same"], true);

        // Diff
        let diff_res = compare_files(CompareFilesIn {
            left_path: f1.to_str().unwrap().into(),
            right_path: f3.to_str().unwrap().into(),
        }).unwrap();
        assert_eq!(diff_res["ok"], true);
        assert_eq!(diff_res["same"], false);
        assert_eq!(diff_res["engine"], "diffy");
        assert!(diff_res["diff"].as_str().unwrap().contains("line changed"));
    }

    #[test]
    fn test_compare_files_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let b1 = tmp.path().join("bin1.dat");
        let b2 = tmp.path().join("bin2.dat");

        std::fs::write(&b1, &[0u8, 1, 2, 3, 0, 5]).unwrap();
        std::fs::write(&b2, &[0u8, 1, 9, 9, 0, 5, 8]).unwrap();

        let res = compare_files(CompareFilesIn {
            left_path: b1.to_str().unwrap().into(),
            right_path: b2.to_str().unwrap().into(),
        }).unwrap();
        assert_eq!(res["ok"], true);
        assert_eq!(res["same"], false);
        assert_eq!(res["engine"], "binary");
    }
}
