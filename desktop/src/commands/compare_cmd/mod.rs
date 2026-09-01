mod ops;

use serde_json::Value;
use tauri::AppHandle;

pub use ops::{CompareDirsIn, CompareFilesIn};

#[tauri::command]
pub async fn compare_dirs(app: AppHandle, input: CompareDirsIn) -> Result<Value, String> {
    ops::compare_dirs(app, input).await
}

#[tauri::command]
pub fn compare_files(input: CompareFilesIn) -> Result<Value, String> {
    ops::compare_files(input)
}
