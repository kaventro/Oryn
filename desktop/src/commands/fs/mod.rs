mod read;
mod transfer;
mod write;

use serde_json::Value;
use tauri::AppHandle;

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
