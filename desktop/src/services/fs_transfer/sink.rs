use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::services::fs_transfer::TransferControl;

/// Progress/cancellation boundary for the copy engine.
///
/// The engine talks to this trait instead of `AppHandle`, so transfer logic
/// is unit-testable without a Tauri runtime and the UI wiring can change
/// without touching the engine.
pub trait ProgressSink: Send + Sync {
    fn emit(&self, payload: Value);
    fn is_aborted(&self) -> bool;
}

/// Production sink: emits `fs:copyProgress` events and reads the abort flag
/// from Tauri managed state.
pub struct AppSink<'a>(pub &'a AppHandle);

impl ProgressSink for AppSink<'_> {
    fn emit(&self, payload: Value) {
        let _ = self.0.emit("fs:copyProgress", payload);
    }

    fn is_aborted(&self) -> bool {
        self.0.state::<TransferControl>().is_aborted()
    }
}
