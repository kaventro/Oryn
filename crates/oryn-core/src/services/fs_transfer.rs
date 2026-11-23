use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod conflicts;
mod engine;
pub mod sink;

use engine::{should_copy_instead_of_rename, CopyEngine};
pub use engine::{CopyError, CopyStats, OverwritePolicy};
use sink::ProgressSink;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_kept: Option<bool>,
}

impl TransferOut {
    pub fn success(stats: CopyStats) -> Self {
        Self {
            ok: true,
            cancelled: None,
            error: None,
            copied: Some(stats.copied + stats.symlinks),
            skipped: Some(stats.skipped_existing + stats.symlinks_skipped),
            src_kept: None,
        }
    }

    pub fn cancelled() -> Self {
        Self {
            ok: false,
            cancelled: Some(true),
            error: None,
            copied: None,
            skipped: None,
            src_kept: None,
        }
    }

    pub fn failed(error: String) -> Self {
        Self {
            ok: false,
            cancelled: None,
            error: Some(error),
            copied: None,
            skipped: None,
            src_kept: None,
        }
    }

    pub fn from_engine(result: Result<CopyStats, CopyError>) -> Self {
        match result {
            Ok(stats) => Self::success(stats),
            Err(CopyError::Aborted) => Self::cancelled(),
            Err(CopyError::Failed(error)) => Self::failed(error),
        }
    }
}

pub fn copy(
    sink: &dyn ProgressSink,
    src: &Path,
    dst: &Path,
    policy: OverwritePolicy,
) -> TransferOut {
    TransferOut::from_engine(CopyEngine::new(sink, policy, src).run(src, dst))
}

pub fn move_path(
    sink: &dyn ProgressSink,
    src: &Path,
    dst: &Path,
    policy: OverwritePolicy,
) -> TransferOut {
    let dst_exists = fs::symlink_metadata(dst).is_ok();
    if dst_exists && policy == OverwritePolicy::Skip {
        let mut out = TransferOut::success(CopyStats {
            skipped_existing: 1,
            ..CopyStats::default()
        });
        out.src_kept = Some(true);
        return out;
    }

    match fs::rename(src, dst) {
        Ok(()) => {
            return TransferOut::success(CopyStats {
                copied: 1,
                ..CopyStats::default()
            })
        }
        Err(error) if !should_copy_instead_of_rename(&error) && !dst_exists => {
            return TransferOut::failed(error.to_string());
        }
        Err(_) => {}
    }

    let result = CopyEngine::new(sink, policy, src)
        .run(src, dst)
        .and_then(|stats| {
            if stats.is_complete() {
                if src.is_dir() {
                    fs::remove_dir_all(src).map_err(CopyError::from)?;
                } else {
                    fs::remove_file(src).map_err(CopyError::from)?;
                }
            }
            Ok(stats)
        });
    let kept = matches!(&result, Ok(stats) if !stats.is_complete());
    let mut out = TransferOut::from_engine(result);
    if kept {
        out.src_kept = Some(true);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_control_lifecycle() {
        let control = TransferControl::default();
        assert!(!control.is_aborted());
        control.request_abort();
        assert!(control.is_aborted());
        control.reset();
        assert!(!control.is_aborted());
    }

    #[test]
    fn test_transfer_out_states() {
        let success = TransferOut::success(CopyStats {
            copied: 5,
            symlinks: 1,
            skipped_existing: 2,
            symlinks_skipped: 1,
        });
        assert!(success.ok);
        assert_eq!(success.copied, Some(6));
        assert_eq!(success.skipped, Some(3));
        assert!(success.cancelled.is_none());

        let cancelled = TransferOut::cancelled();
        assert!(!cancelled.ok);
        assert_eq!(cancelled.cancelled, Some(true));

        let failed = TransferOut::failed("disk full".into());
        assert!(!failed.ok);
        assert_eq!(failed.error.as_deref(), Some("disk full"));

        let from_engine_aborted = TransferOut::from_engine(Err(CopyError::Aborted));
        assert_eq!(from_engine_aborted.cancelled, Some(true));

        let from_engine_failed =
            TransferOut::from_engine(Err(CopyError::Failed("io error".into())));
        assert_eq!(from_engine_failed.error.as_deref(), Some("io error"));
    }
}
