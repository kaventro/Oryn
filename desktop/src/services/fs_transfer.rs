use std::path::{Path, PathBuf};

use oryn_core::services::fs_transfer::sink::ProgressSink;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::services::ServiceResult;

pub use oryn_core::services::fs_transfer::{
    conflicts, CopyError, CopyStats, OverwritePolicy, TransferControl, TransferOut,
};

type TransferFn = fn(&dyn ProgressSink, &Path, &Path, OverwritePolicy) -> TransferOut;

pub struct AppSink<'a, R: Runtime>(pub &'a AppHandle<R>);

impl<R: Runtime> ProgressSink for AppSink<'_, R> {
    fn emit(&self, payload: Value) {
        let _ = self.0.emit("fs:copyProgress", payload);
    }

    fn is_aborted(&self) -> bool {
        self.0.state::<TransferControl>().is_aborted()
    }
}

async fn run_blocking<R: Runtime>(
    app: AppHandle<R>,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
    op: TransferFn,
) -> ServiceResult<TransferOut> {
    app.state::<TransferControl>().reset();
    let src = PathBuf::from(src);
    let dst = PathBuf::from(dst);
    tokio::task::spawn_blocking(move || op(&AppSink(&app), &src, &dst, policy))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub async fn copy<R: Runtime>(
    app: AppHandle<R>,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    run_blocking(app, src, dst, policy, oryn_core::services::fs_transfer::copy).await
}

pub async fn move_path<R: Runtime>(
    app: AppHandle<R>,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    run_blocking(app, src, dst, policy, oryn_core::services::fs_transfer::move_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> AppHandle<tauri::test::MockRuntime> {
        let app = tauri::test::mock_builder()
            .manage(TransferControl::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app");
        app.handle().clone()
    }

    #[tokio::test]
    async fn copies_and_moves_through_the_app_sink() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.txt");
        std::fs::write(&src, b"hello").unwrap();
        let copied = tmp.path().join("b.txt");
        let moved = tmp.path().join("c.txt");
        let app = test_app();

        let out = copy(
            app.clone(),
            &src.to_string_lossy(),
            &copied.to_string_lossy(),
            OverwritePolicy::Overwrite,
        )
        .await
        .unwrap();
        assert!(out.ok);
        assert_eq!(std::fs::read(&copied).unwrap(), b"hello");

        let out = move_path(
            app.clone(),
            &copied.to_string_lossy(),
            &moved.to_string_lossy(),
            OverwritePolicy::Overwrite,
        )
        .await
        .unwrap();
        assert!(out.ok);
        assert!(!copied.exists());
        assert!(moved.exists());
    }

    #[test]
    fn the_sink_reports_abort_requests() {
        let app = test_app();
        assert!(!AppSink(&app).is_aborted());
        app.state::<TransferControl>().request_abort();
        assert!(AppSink(&app).is_aborted());
    }
}
