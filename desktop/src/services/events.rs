use oryn_core::events::EventSink;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

pub struct TauriEvents(pub AppHandle);

impl EventSink for TauriEvents {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}
