mod core;

pub use core::ZipExtractIn;
use serde_json::Value;

#[tauri::command]
pub fn zip_extract(input: ZipExtractIn) -> Result<Value, String> {
    core::zip_extract(input)
}
