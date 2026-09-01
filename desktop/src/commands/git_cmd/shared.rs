use std::process::Stdio;
use tokio::process::Command;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirPathIn {
    pub dir_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoPathIn {
    pub repo_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogIn {
    pub repo_path: String,
    pub max_count: Option<u32>,
    pub file_path: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashIn {
    pub repo_path: String,
    pub message: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRestoreIn {
    pub repo_path: String,
    pub file_path: String,
    pub staged: Option<bool>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAddIn {
    pub repo_path: String,
    pub files: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitIn {
    pub repo_path: String,
    pub message: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCheckoutIn {
    pub repo_path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub is_file: Option<bool>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStageIn {
    pub repo_path: String,
    pub file_path: String,
    pub stage: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBlameIn {
    pub repo_path: String,
    pub file_path: String,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffIn {
    pub repo_path: String,
    pub ref1: Option<String>,
    pub ref2: Option<String>,
    pub file_path: Option<String>,
}

pub async fn git(args: &[&str], cwd: &str) -> std::io::Result<std::process::Output> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd.output().await
}
