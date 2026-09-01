use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use super::shared::{binary_differ, build_dir_map, same_inode, sort_rows, DirEnt};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareDirsIn {
    pub left_path: String,
    pub right_path: String,
}

pub async fn compare_dirs(app: AppHandle, input: CompareDirsIn) -> Result<Value, String> {
    let left = PathBuf::from(&input.left_path);
    let right = PathBuf::from(&input.right_path);
    let app2 = app.clone();
    tokio::task::spawn_blocking(move || compare_dirs_blocking(&app2, &left, &right))
        .await
        .map_err(|e| e.to_string())?
}

fn compare_dirs_blocking(app: &AppHandle, left: &Path, right: &Path) -> Result<Value, String> {
    let left_map = build_dir_map(left)?;
    let right_map = build_dir_map(right)?;

    let mut only_left: Vec<Value> = Vec::new();
    let mut only_right: Vec<Value> = Vec::new();
    let mut same: Vec<Value> = Vec::new();
    let mut to_hash: Vec<(String, DirEnt, DirEnt, &'static str)> = Vec::new();

    for (rel, l) in &left_map {
        match right_map.get(rel) {
            None => {
                let mut v = serde_json::to_value(l).map_err(|e| e.to_string())?;
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("side".into(), json!("left"));
                }
                only_left.push(v);
            }
            Some(r) => {
                if l.is_dir && r.is_dir {
                    same.push(json!({"rel": rel}));
                } else if l.is_dir != r.is_dir {
                    to_hash.push((rel.clone(), l.clone(), r.clone(), "type-mismatch"));
                } else if l.size != r.size {
                    to_hash.push((rel.clone(), l.clone(), r.clone(), "different"));
                } else if (l.mtime - r.mtime).abs() < 2000.0 {
                    same.push(json!({"rel": rel}));
                } else {
                    to_hash.push((rel.clone(), l.clone(), r.clone(), "need-hash"));
                }
            }
        }
    }
    for (rel, r) in &right_map {
        if !left_map.contains_key(rel) {
            let mut v = serde_json::to_value(r).map_err(|e| e.to_string())?;
            if let Some(obj) = v.as_object_mut() {
                obj.insert("side".into(), json!("right"));
            }
            only_right.push(v);
        }
    }

    let _ = app.emit(
        "compare:update",
        json!({
            "type": "partial",
            "onlyLeft": sort_rows(only_left.clone()),
            "onlyRight": sort_rows(only_right.clone()),
            "same": sort_rows(same.clone()),
            "different": [],
        }),
    );

    let mut different: Vec<Value> = Vec::new();
    for (rel, l, r, reason) in &to_hash {
        if *reason == "type-mismatch" || *reason == "different" {
            different.push(json!({
                "rel": rel,
                "left": serde_json::to_value(l).map_err(|e| e.to_string())?,
                "right": serde_json::to_value(r).map_err(|e| e.to_string())?,
                "reason": reason
            }));
        }
    }

    let need: Vec<_> = to_hash
        .into_iter()
        .filter(|(_, _, _, r)| *r == "need-hash")
        .collect();
    let total = need.len();
    for (i, (rel, l, r, _)) in need.into_iter().enumerate() {
        if same_inode(&l, &r) {
            same.push(json!({"rel": rel}));
            continue;
        }
        let diff = binary_differ(Path::new(&l.full), Path::new(&r.full)).unwrap_or(true);
        if diff {
            different.push(json!({
                "rel": rel,
                "left": serde_json::to_value(&l).map_err(|e| e.to_string())?,
                "right": serde_json::to_value(&r).map_err(|e| e.to_string())?,
                "reason": "modified"
            }));
        } else {
            same.push(json!({"rel": rel}));
        }
        let done = i + 1;
        if done % 48 == 0 || done == total {
            let _ = app.emit(
                "compare:update",
                json!({"type": "progress", "done": done, "total": total}),
            );
        }
    }

    let ol = only_left.len();
    let ort = only_right.len();
    let dn = different.len();
    let sn = same.len();
    let stats = json!({
        "onlyLeft": ol,
        "onlyRight": ort,
        "different": dn,
        "same": sn,
        "total": left_map.len() + ort
    });
    let body = json!({
        "ok": true,
        "leftPath": left.to_string_lossy().to_string(),
        "rightPath": right.to_string_lossy().to_string(),
        "onlyLeft": sort_rows(only_left),
        "onlyRight": sort_rows(only_right),
        "different": sort_rows(different),
        "same": sort_rows(same),
        "stats": stats
    });
    let mut emit_obj: Map<String, Value> = body.as_object().cloned().unwrap_or_default();
    emit_obj.insert("type".into(), json!("done"));
    let _ = app.emit("compare:update", Value::Object(emit_obj));
    Ok(body)
}
