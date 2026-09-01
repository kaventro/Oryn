use serde_json::{json, Value};
use tokio::process::Command;

use super::shared::{
    git, GitAddIn, GitCheckoutIn, GitCommitIn, GitRestoreIn, GitStageIn, GitStashIn, RepoPathIn,
};

pub async fn git_add(input: GitAddIn) -> Result<Value, String> {
    let mut args = vec!["add".to_string(), "--".to_string()];
    args.extend(input.files);
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = Command::new("git")
        .args(&args_ref)
        .current_dir(&input.repo_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if o.status.success() {
        Ok(json!({"ok": true}))
    } else {
        Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string()}))
    }
}

pub async fn git_commit(input: GitCommitIn) -> Result<Value, String> {
    let o = Command::new("git")
        .args(["commit", "-m", &input.message])
        .current_dir(&input.repo_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if o.status.success() {
        Ok(json!({"ok": true, "output": String::from_utf8_lossy(&o.stdout).to_string()}))
    } else {
        Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string()}))
    }
}

pub async fn git_push(input: RepoPathIn) -> Result<Value, String> {
    let o = Command::new("git")
        .arg("push")
        .current_dir(&input.repo_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    Ok(json!({
        "ok": o.status.success(),
        "output": out,
        "error": if o.status.success() { Value::Null } else { json!(out.trim()) }
    }))
}

pub async fn git_pull(input: RepoPathIn) -> Result<Value, String> {
    let o = Command::new("git")
        .arg("pull")
        .current_dir(&input.repo_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    Ok(json!({
        "ok": o.status.success(),
        "output": out,
        "error": if o.status.success() { Value::Null } else { json!(out.trim()) }
    }))
}

pub async fn git_checkout(input: GitCheckoutIn) -> Result<Value, String> {
    let git_ref = input.git_ref.as_str();
    // Refuse a ref that would be parsed as an option (e.g. "--orphan"); branch
    // names and hashes never legitimately start with '-'.
    if git_ref.starts_with('-') {
        return Ok(json!({"ok": false, "error": "invalid git ref"}));
    }
    let o = if input.is_file.unwrap_or(false) {
        git(&["checkout", "--", git_ref], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    } else {
        git(&["checkout", git_ref], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    };

    if o.status.success() {
        Ok(json!({"ok": true}))
    } else {
        Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string()}))
    }
}

pub async fn git_stage_file(input: GitStageIn) -> Result<Value, String> {
    let mut file_path = input.file_path.trim().to_string();
    if file_path.starts_with(&input.repo_path) {
        file_path = file_path[input.repo_path.len()..]
            .trim_start_matches(['/', '\\'])
            .to_string();
    } else {
        file_path = file_path.trim_start_matches(['/', '\\']).to_string();
    }
    let o = if input.stage {
        git(&["add", "--", &file_path], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    } else {
        git(&["restore", "--staged", "--", &file_path], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    };

    if o.status.success() {
        Ok(json!({"ok": true}))
    } else {
        Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string()}))
    }
}

pub async fn git_restore(input: GitRestoreIn) -> Result<Value, String> {
    let mut file_path = input.file_path.trim().to_string();
    if file_path.starts_with(&input.repo_path) {
        file_path = file_path[input.repo_path.len()..]
            .trim_start_matches(['/', '\\'])
            .to_string();
    } else {
        file_path = file_path.trim_start_matches(['/', '\\']).to_string();
    }
    let o = if input.staged.unwrap_or(false) {
        git(&["restore", "--staged", "--", &file_path], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    } else {
        git(&["restore", "--", &file_path], &input.repo_path)
            .await
            .map_err(|e| e.to_string())?
    };

    if o.status.success() {
        Ok(json!({"ok": true}))
    } else {
        Ok(json!({"ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string()}))
    }
}

pub async fn git_stash(input: GitStashIn) -> Result<Value, String> {
    let mut args = vec!["stash".to_string(), "push".to_string(), "-u".to_string()];
    if let Some(msg) = input.message {
        if !msg.trim().is_empty() {
            args.push("-m".to_string());
            args.push(msg);
        }
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = git(&args_ref, &input.repo_path)
        .await
        .map_err(|e| e.to_string())?;

    let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    Ok(json!({
        "ok": o.status.success(),
        "output": out,
        "error": if o.status.success() { Value::Null } else { json!(out.trim()) }
    }))
}

pub async fn git_stash_pop(input: RepoPathIn) -> Result<Value, String> {
    let o = git(&["stash", "pop"], &input.repo_path)
        .await
        .map_err(|e| e.to_string())?;

    let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    Ok(json!({
        "ok": o.status.success(),
        "output": out,
        "error": if o.status.success() { Value::Null } else { json!(out.trim()) }
    }))
}

pub async fn git_stash_list(input: RepoPathIn) -> Result<Value, String> {
    let o = git(&["stash", "list"], &input.repo_path)
        .await
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Ok(json!({
            "ok": false,
            "error": String::from_utf8_lossy(&o.stderr).to_string(),
            "stashes": []
        }));
    }

    let stashes: Vec<String> = String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(json!({
        "ok": true,
        "stashes": stashes
    }))
}
