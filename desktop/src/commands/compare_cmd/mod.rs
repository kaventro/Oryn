use std::sync::Arc;

use oryn_core::compare as ops;

use serde_json::Value;
use tauri::AppHandle;

use crate::services::events::TauriEvents;

pub use ops::{CompareDirsIn, CompareFilesIn};

#[tauri::command]
pub async fn compare_dirs(app: AppHandle, input: CompareDirsIn) -> Result<Value, String> {
    ops::compare_dirs(Arc::new(TauriEvents(app)), input).await
}

#[tauri::command]
pub fn compare_files(input: CompareFilesIn) -> Result<Value, String> {
    ops::compare_files(input)
}
