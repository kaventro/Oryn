use tauri::AppHandle;

use crate::services::fs_transfer::{self, conflicts, OverwritePolicy, TransferOut};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyMoveIn {
    pub src: String,
    pub dst: String,
    /// "overwrite" (default) | "skip"
    pub policy: Option<String>,
}

impl CopyMoveIn {
    fn policy(&self) -> OverwritePolicy {
        OverwritePolicy::parse(self.policy.as_deref())
    }
}

pub async fn fs_copy(app: AppHandle, input: CopyMoveIn) -> Result<TransferOut, String> {
    let policy = input.policy();
    fs_transfer::copy(app, &input.src, &input.dst, policy)
        .await
        .map_err(|e| e.to_string())
}

pub async fn fs_move(app: AppHandle, input: CopyMoveIn) -> Result<TransferOut, String> {
    let policy = input.policy();
    fs_transfer::move_path(app, &input.src, &input.dst, policy)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictsIn {
    pub src: String,
    pub dst: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictsOut {
    pub ok: bool,
    pub conflicts: Vec<String>,
    pub truncated: bool,
}

const CONFLICT_SCAN_LIMIT: usize = 50;

pub async fn fs_copy_conflicts(input: ConflictsIn) -> Result<ConflictsOut, String> {
    let src = std::path::PathBuf::from(&input.src);
    let dst = std::path::PathBuf::from(&input.dst);
    let report =
        tokio::task::spawn_blocking(move || conflicts::scan(&src, &dst, CONFLICT_SCAN_LIMIT))
            .await
            .map_err(|e| e.to_string())?;
    Ok(ConflictsOut {
        ok: true,
        conflicts: report.conflicts,
        truncated: report.truncated,
    })
}
