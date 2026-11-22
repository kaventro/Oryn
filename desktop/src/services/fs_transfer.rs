use std::path::PathBuf;

use oryn_core::services::fs_transfer::sink::ProgressSink;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::services::ServiceResult;

pub use oryn_core::services::fs_transfer::{
    conflicts, CopyError, CopyStats, OverwritePolicy, TransferControl, TransferOut,
};

pub struct AppSink<'a>(pub &'a AppHandle);

impl ProgressSink for AppSink<'_> {
    fn emit(&self, payload: Value) {
        let _ = self.0.emit("fs:copyProgress", payload);
    }

    fn is_aborted(&self) -> bool {
        self.0.state::<TransferControl>().is_aborted()
    }
}

async fn run_blocking(
    app: AppHandle,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
    op: fn(&dyn ProgressSink, &std::path::Path, &std::path::Path, OverwritePolicy) -> TransferOut,
) -> ServiceResult<TransferOut> {
    app.state::<TransferControl>().reset();
    let src = PathBuf::from(src);
    let dst = PathBuf::from(dst);
    tokio::task::spawn_blocking(move || op(&AppSink(&app), &src, &dst, policy))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub async fn copy(
    app: AppHandle,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    run_blocking(app, src, dst, policy, oryn_core::services::fs_transfer::copy).await
}

pub async fn move_path(
    app: AppHandle,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    run_blocking(app, src, dst, policy, oryn_core::services::fs_transfer::move_path).await
}
