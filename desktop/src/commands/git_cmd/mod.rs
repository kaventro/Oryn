mod actions;
mod query;
mod shared;

use serde_json::Value;

pub use shared::{
    DirPathIn, GitAddIn, GitBlameIn, GitCheckoutIn, GitCommitIn, GitDiffIn, GitLogIn, GitRestoreIn,
    GitStageIn, GitStashIn, RepoPathIn,
};

#[tauri::command]
pub async fn git_is_repo(input: DirPathIn) -> Result<Value, String> {
    query::git_is_repo(input).await
}

#[tauri::command]
pub async fn git_status(input: RepoPathIn) -> Result<Value, String> {
    query::git_status(input).await
}

#[tauri::command]
pub async fn git_log(input: GitLogIn) -> Result<Value, String> {
    query::git_log(input).await
}

#[tauri::command]
pub async fn git_diff(input: GitDiffIn) -> Result<Value, String> {
    query::git_diff(input).await
}

#[tauri::command]
pub async fn git_blame(input: GitBlameIn) -> Result<Value, String> {
    query::git_blame(input).await
}

#[tauri::command]
pub async fn git_branches(input: RepoPathIn) -> Result<Value, String> {
    query::git_branches(input).await
}

#[tauri::command]
pub async fn git_add(input: GitAddIn) -> Result<Value, String> {
    actions::git_add(input).await
}

#[tauri::command]
pub async fn git_commit(input: GitCommitIn) -> Result<Value, String> {
    actions::git_commit(input).await
}

#[tauri::command]
pub async fn git_push(input: RepoPathIn) -> Result<Value, String> {
    actions::git_push(input).await
}

#[tauri::command]
pub async fn git_pull(input: RepoPathIn) -> Result<Value, String> {
    actions::git_pull(input).await
}

#[tauri::command]
pub async fn git_checkout(input: GitCheckoutIn) -> Result<Value, String> {
    actions::git_checkout(input).await
}

#[tauri::command]
pub async fn git_stage_file(input: GitStageIn) -> Result<Value, String> {
    actions::git_stage_file(input).await
}

#[tauri::command]
pub async fn git_restore(input: GitRestoreIn) -> Result<Value, String> {
    actions::git_restore(input).await
}

#[tauri::command]
pub async fn git_stash(input: GitStashIn) -> Result<Value, String> {
    actions::git_stash(input).await
}

#[tauri::command]
pub async fn git_stash_pop(input: RepoPathIn) -> Result<Value, String> {
    actions::git_stash_pop(input).await
}

#[tauri::command]
pub async fn git_stash_list(input: RepoPathIn) -> Result<Value, String> {
    actions::git_stash_list(input).await
}
