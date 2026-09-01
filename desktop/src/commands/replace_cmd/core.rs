use serde_json::{json, Value};
use std::fs;
use tauri::{AppHandle, Emitter};
use tokio::process::Command as AsyncCommand;

const MAX_REPLACE_FILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(serde::Deserialize)]
pub struct FindReplaceIn {
    pub payload: Value,
}

pub async fn fs_find_replace(app: AppHandle, input: FindReplaceIn) -> Result<Value, String> {
    let payload = input.payload;
    let root_dirs: Vec<String> = payload
        .get("rootDirs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "rootDirs required".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let find_text = payload
        .get("findText")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "findText required".to_string())?
        .to_string();
    if find_text.is_empty() {
        return Ok(json!({"ok": false, "error": "findText is required", "changed": []}));
    }
    let replace_text = payload
        .get("replaceText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let case_sensitive = payload
        .get("caseSensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let use_regex = payload
        .get("useRegex")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let dry_run = payload
        .get("dryRun")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let file_pattern = payload
        .get("filePattern")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut matched: Vec<String> = Vec::new();
    for root in &root_dirs {
        let mut cmd = AsyncCommand::new("rg");
        cmd.arg("-l");
        if !case_sensitive {
            cmd.arg("-i");
        }
        if !use_regex {
            cmd.arg("-F");
        }
        for p in file_pattern.split(';') {
            let p = p.trim();
            if !p.is_empty() {
                let g = if p.contains('/') {
                    p.to_string()
                } else {
                    format!("**/{}", p)
                };
                cmd.args(["--iglob", &g]);
            }
        }
        cmd.arg("--");
        cmd.arg(&find_text);
        cmd.arg(root);
        let out = match cmd.output().await {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(json!({
                    "ok": false,
                    "error": "ripgrep (rg) was not found in PATH; install it to use Find & Replace",
                    "changed": []
                }));
            }
            Err(e) => {
                return Ok(json!({"ok": false, "error": e.to_string(), "changed": []}));
            }
        };
        if !out.status.success() && out.status.code() != Some(1) {
            continue;
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let t = line.trim().to_string();
            if !t.is_empty() {
                matched.push(t);
            }
        }
    }
    matched.sort();
    matched.dedup();
    if matched.is_empty() {
        return Ok(json!({"ok": true, "changed": [], "count": 0, "dryRun": dry_run}));
    }

    let mut changed: Vec<String> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();
    for (i, file_path) in matched.iter().enumerate() {
        let _ = app.emit(
            "fs:replaceProgress",
            json!({"file": file_path, "index": i + 1, "total": matched.len()}),
        );
        if dry_run {
            changed.push(file_path.clone());
            continue;
        }
        // Guard against loading a huge matched file entirely into memory.
        if fs::metadata(file_path).map(|m| m.len()).unwrap_or(0) > MAX_REPLACE_FILE_BYTES {
            errors.push(json!({"file": file_path, "error": "file too large for in-place replace"}));
            continue;
        }
        let original = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(json!({"file": file_path, "error": e.to_string()}));
                continue;
            }
        };
        let new_content = match apply_replacement(
            &original,
            &find_text,
            &replace_text,
            case_sensitive,
            use_regex,
        ) {
            Ok(c) => c,
            Err(e) => return Ok(json!({"ok": false, "error": e})),
        };
        if new_content != original {
            if let Err(e) = fs::write(file_path, new_content) {
                errors.push(json!({"file": file_path, "error": e.to_string()}));
            } else {
                changed.push(file_path.clone());
            }
        }
    }
    Ok(json!({
        "ok": true,
        "changed": changed,
        "count": changed.len(),
        "errors": errors,
        "dryRun": dry_run
    }))
}

fn apply_replacement(
    original: &str,
    find_text: &str,
    replace_text: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Result<String, String> {
    if use_regex {
        let re = regex::RegexBuilder::new(find_text)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(re.replace_all(original, replace_text).to_string())
    } else if case_sensitive {
        Ok(original.replace(find_text, replace_text))
    } else {
        let re = regex::RegexBuilder::new(&regex::escape(find_text))
            .case_insensitive(true)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(re
            .replace_all(original, regex::NoExpand(replace_text))
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::apply_replacement;

    #[test]
    fn case_sensitive_literal() {
        assert_eq!(
            apply_replacement("Foo foo FOO", "foo", "bar", true, false).unwrap(),
            "Foo bar FOO"
        );
    }

    #[test]
    fn case_insensitive_literal() {
        assert_eq!(
            apply_replacement("Foo foo FOO", "foo", "bar", false, false).unwrap(),
            "bar bar bar"
        );
    }

    #[test]
    fn case_insensitive_handles_length_changing_unicode_without_panic() {
        let out = apply_replacement("İ foo K bar İ", "FOO", "baz", false, false).unwrap();
        assert_eq!(out, "İ baz K bar İ");
    }

    #[test]
    fn literal_replacement_does_not_expand_dollar_groups() {
        // In literal mode, `$1` in the replacement must be inserted verbatim.
        assert_eq!(
            apply_replacement("hello", "hello", "$1world", false, false).unwrap(),
            "$1world"
        );
    }

    #[test]
    fn regex_mode_expands_capture_groups() {
        assert_eq!(
            apply_replacement(r"key=val", r"(\w+)=(\w+)", "$2=$1", true, true).unwrap(),
            "val=key"
        );
    }

    #[test]
    fn invalid_regex_returns_err() {
        assert!(apply_replacement("x", "(unclosed", "y", true, true).is_err());
    }
}
