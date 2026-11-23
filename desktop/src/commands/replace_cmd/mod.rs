use std::sync::Arc;

use oryn_core::replace as core;

use serde_json::Value;
use tauri::AppHandle;

use crate::services::events::TauriEvents;

pub use core::FindReplaceIn;

#[tauri::command]
pub async fn fs_find_replace(app: AppHandle, input: FindReplaceIn) -> Result<Value, String> {
    core::fs_find_replace(Arc::new(TauriEvents(app)), input).await
}
