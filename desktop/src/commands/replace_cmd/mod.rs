mod core;

use serde_json::Value;
use tauri::AppHandle;

pub use core::FindReplaceIn;

#[tauri::command]
pub async fn fs_find_replace(app: AppHandle, input: FindReplaceIn) -> Result<Value, String> {
    core::fs_find_replace(app, input).await
}
