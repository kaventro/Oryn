pub mod core;

use serde_json::Value;

#[tauri::command]
pub fn archive_create(input: core::CreateArchiveIn) -> Result<Value, String> {
    core::create_archive(input)
}

#[tauri::command]
pub fn archive_extract(input: core::ExtractArchiveIn) -> Result<Value, String> {
    core::extract_archive(input)
}

#[tauri::command]
pub fn archive_list(input: core::ListArchiveIn) -> Result<Value, String> {
    core::list_archive_entries(input)
}
