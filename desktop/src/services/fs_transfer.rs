use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

use crate::services::ServiceResult;

pub mod conflicts;
mod engine;
pub mod sink;

use engine::{should_copy_instead_of_rename, CopyEngine};
pub use engine::{CopyError, CopyStats, OverwritePolicy};
use sink::AppSink;

/// Cancellation flag for the transfer in flight, held in Tauri managed state
/// (`app.manage(TransferControl::default())`).
#[derive(Default)]
pub struct TransferControl {
    abort: AtomicBool,
}

impl TransferControl {
    pub fn request_abort(&self) {
        self.abort.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.abort.store(false, Ordering::SeqCst);
    }

    pub fn is_aborted(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }
}

fn control(app: &AppHandle) -> tauri::State<'_, TransferControl> {
    app.state::<TransferControl>()
}

/// Transfer outcome envelope. Unlike plain commands, a transfer has
/// non-error outcomes (cancelled, partially skipped), so they travel in the
/// payload instead of `Err`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferOut {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copied: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<u64>,
    /// Set on a move that skipped items: the source was kept to avoid
    /// deleting data that never reached the destination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_kept: Option<bool>,
}

impl TransferOut {
    fn success(stats: CopyStats) -> Self {
        Self {
            ok: true,
            cancelled: None,
            error: None,
            copied: Some(stats.copied + stats.symlinks),
            skipped: Some(stats.skipped_existing + stats.symlinks_skipped),
            src_kept: None,
        }
    }

    fn cancelled() -> Self {
        Self {
            ok: false,
            cancelled: Some(true),
            error: None,
            copied: None,
            skipped: None,
            src_kept: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            ok: false,
            cancelled: None,
            error: Some(error),
            copied: None,
            skipped: None,
            src_kept: None,
        }
    }

    fn from_engine(result: Result<CopyStats, CopyError>) -> Self {
        match result {
            Ok(stats) => Self::success(stats),
            Err(CopyError::Aborted) => Self::cancelled(),
            Err(CopyError::Failed(error)) => Self::failed(error),
        }
    }
}

pub async fn copy(
    app: AppHandle,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    control(&app).reset();
    let src_path = PathBuf::from(src);
    let dst_path = PathBuf::from(dst);
    let app_handle = app.clone();

    let result = tokio::task::spawn_blocking(move || {
        let sink = AppSink(&app_handle);
        CopyEngine::new(&sink, policy, &src_path).run(&src_path, &dst_path)
    })
    .await;

    match result {
        Ok(inner) => Ok(TransferOut::from_engine(inner)),
        Err(error) => Err(anyhow::anyhow!(error.to_string())),
    }
}

pub async fn move_path(
    app: AppHandle,
    src: &str,
    dst: &str,
    policy: OverwritePolicy,
) -> ServiceResult<TransferOut> {
    control(&app).reset();

    // Fast path: same-volume rename. Refuse to clobber when the policy says
    // skip; rename() would replace the destination silently.
    let dst_exists = fs::symlink_metadata(dst).is_ok();
    if dst_exists && policy == OverwritePolicy::Skip {
        let mut out = TransferOut::success(CopyStats {
            skipped_existing: 1,
            ..CopyStats::default()
        });
        out.src_kept = Some(true);
        return Ok(out);
    }

    if let Err(error) = fs::rename(src, dst) {
        if !should_copy_instead_of_rename(&error) && !dst_exists {
            return Ok(TransferOut::failed(error.to_string()));
        }
        // Cross-device move, or rename refused because the destination
        // exists (e.g. non-empty dir): fall through to copy + delete.
    } else {
        return Ok(TransferOut::success(CopyStats {
            copied: 1,
            ..CopyStats::default()
        }));
    }

    let src_path = PathBuf::from(src);
    let dst_path = PathBuf::from(dst);
    let app_handle = app.clone();

    let result = tokio::task::spawn_blocking(move || {
        let sink = AppSink(&app_handle);
        let stats = CopyEngine::new(&sink, policy, &src_path).run(&src_path, &dst_path)?;

        // Delete the source only when everything was transferred; with
        // skipped items the source is the sole copy of that data.
        if stats.is_complete() {
            if src_path.is_dir() {
                fs::remove_dir_all(&src_path).map_err(CopyError::from)?;
            } else {
                fs::remove_file(&src_path).map_err(CopyError::from)?;
            }
        }
        Ok::<CopyStats, CopyError>(stats)
    })
    .await;

    match result {
        Ok(inner) => {
            let kept = matches!(&inner, Ok(stats) if !stats.is_complete());
            let mut out = TransferOut::from_engine(inner);
            if kept {
                out.src_kept = Some(true);
            }
            Ok(out)
        }
        Err(error) => Err(anyhow::anyhow!(error.to_string())),
    }
}
