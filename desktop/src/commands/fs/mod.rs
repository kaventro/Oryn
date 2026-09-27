mod read;
mod transfer;
mod write;

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

use crate::commands::response::Ack;
use crate::services::fs_props::StatPropsOut;
use crate::services::fs_transfer::{TransferControl, TransferOut};
use crate::services::fs_vfs::DirListing;

pub use read::{
    ChecksumIn, ChecksumOut, DirSizeIn, DirSizeOut, ReadDirIn, ReadFileIn, StatIn, TextProbe,
};
pub use transfer::{ConflictsIn, ConflictsOut, CopyMoveIn};
pub use write::{
    CompressIn, CreateFileIn, DeleteIn, ExtractIn, MkdirIn, RenameIn, WriteFileTextIn, ZipCreated,
    ZipIn,
};

#[tauri::command]
pub fn config_load() -> Result<Value, String> {
    read::config_load()
}

#[tauri::command]
pub fn fs_read_dir(input: ReadDirIn) -> Result<DirListing, String> {
    read::fs_read_dir(input)
}

#[tauri::command]
pub async fn fs_read_flat_branch(input: ReadDirIn) -> Result<DirListing, String> {
    read::fs_read_flat_branch(input).await
}

#[tauri::command]
pub fn fs_stat_props(input: StatIn) -> Result<StatPropsOut, String> {
    read::fs_stat_props(input)
}

#[tauri::command]
pub fn fs_rename(input: RenameIn) -> Result<Ack, String> {
    write::fs_rename(input)
}

#[tauri::command]
pub fn fs_delete(input: DeleteIn) -> Result<Ack, String> {
    write::fs_delete(input)
}

#[tauri::command]
pub fn fs_read_file_text(input: ReadFileIn) -> Result<String, String> {
    read::fs_read_file_text(input)
}

#[tauri::command]
pub fn fs_probe_text(input: ReadFileIn) -> Result<TextProbe, String> {
    read::fs_probe_text(input)
}

#[tauri::command]
pub fn fs_read_office(input: ReadFileIn) -> Result<crate::services::fs_office::OfficeDoc, String> {
    read::fs_read_office(input)
}

#[tauri::command]
pub fn fs_read_media_data_url(input: ReadFileIn) -> Result<String, String> {
    read::fs_read_media_data_url(input)
}

/// Grants the asset protocol access to one preview file the UI is about to show.
/// The static scope stays limited to app-owned directories; renderer-supplied
/// paths are not readable until this check allows that exact file.
#[tauri::command]
pub fn fs_grant_preview_asset<R: Runtime>(app: AppHandle<R>, input: ReadFileIn) -> Result<String, String> {
    let canonical = crate::services::preview_asset::validate_preview_asset(&input.path)?;
    app.asset_protocol_scope()
        .allow_file(&canonical)
        .map_err(|e| e.to_string())?;
    Ok(input.path)
}

#[tauri::command]
pub fn fs_mkdir(input: MkdirIn) -> Result<Ack, String> {
    write::fs_mkdir(input)
}

#[tauri::command]
pub fn fs_create_file(input: CreateFileIn) -> Result<Ack, String> {
    write::fs_create_file(input)
}

#[tauri::command]
pub fn fs_write_file_text(input: WriteFileTextIn) -> Result<Ack, String> {
    write::fs_write_file_text(input)
}

#[tauri::command]
pub fn fs_compress_zip(input: ZipIn) -> Result<ZipCreated, String> {
    write::fs_compress_zip(input)
}

#[tauri::command]
pub fn fs_compress(input: CompressIn) -> Result<ZipCreated, String> {
    write::fs_compress(input)
}

#[tauri::command]
pub fn fs_extract(input: ExtractIn) -> Result<Ack, String> {
    write::fs_extract(input)
}

#[tauri::command]
pub async fn fs_copy(app: AppHandle, input: CopyMoveIn) -> Result<TransferOut, String> {
    transfer::fs_copy(app, input).await
}

#[tauri::command]
pub async fn fs_move(app: AppHandle, input: CopyMoveIn) -> Result<TransferOut, String> {
    transfer::fs_move(app, input).await
}

#[tauri::command]
pub async fn fs_copy_conflicts(input: ConflictsIn) -> Result<ConflictsOut, String> {
    transfer::fs_copy_conflicts(input).await
}

#[tauri::command]
pub fn fs_cancel_copy(control: tauri::State<'_, TransferControl>) {
    control.request_abort();
}

#[tauri::command]
pub async fn fs_get_dir_size(input: DirSizeIn) -> Result<DirSizeOut, String> {
    read::fs_get_dir_size(input).await
}

#[tauri::command]
pub async fn fs_analyze_dir(
    input: DirSizeIn,
) -> Result<crate::services::fs_size::DiskSpaceAnalysis, String> {
    read::fs_analyze_dir(input).await
}

#[tauri::command]
pub async fn fs_scan_duplicates(
    input: read::DuplicatesIn,
) -> Result<crate::services::fs_duplicates::DuplicateScanResult, String> {
    read::fs_scan_duplicates(input).await
}

#[tauri::command]
pub async fn fs_checksum(input: ChecksumIn) -> Result<ChecksumOut, String> {
    read::fs_checksum(input).await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchDirsIn {
    pub paths: Vec<String>,
}

#[tauri::command]
pub fn fs_watch_dirs(
    watcher: tauri::State<'_, std::sync::Arc<crate::services::fs_watcher::FsWatcherService>>,
    input: WatchDirsIn,
) -> Result<Ack, String> {
    watcher.watch_dirs(input.paths)?;
    Ok(Ack { ok: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> AppHandle<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app")
            .handle()
            .clone()
    }

    #[test]
    fn grants_one_preview_file() {
        let tmp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        std::fs::write(tmp.path(), b"png").unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let granted = fs_grant_preview_asset(
            test_app(),
            ReadFileIn {
                path: path.clone(),
                max_bytes: None,
            },
        )
        .unwrap();

        assert_eq!(granted, path);
    }

    #[test]
    fn refuses_a_preview_grant_for_a_non_media_path() {
        let err = fs_grant_preview_asset(
            test_app(),
            ReadFileIn {
                path: String::new(),
                max_bytes: None,
            },
        )
        .unwrap_err();

        assert!(err.contains("empty"));
    }
}
