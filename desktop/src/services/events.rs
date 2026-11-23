use oryn_core::events::EventSink;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

pub struct TauriEvents<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> EventSink for TauriEvents<R> {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Listener;

    #[test]
    fn forwards_events_to_the_app() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app");
        let (tx, rx) = std::sync::mpsc::channel();
        app.listen("fs:change", move |event| {
            let _ = tx.send(event.payload().to_string());
        });
        TauriEvents(app.handle().clone()).emit("fs:change", serde_json::json!({"path": "/tmp"}));
        let payload = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("event delivered");
        assert!(payload.contains("/tmp"));
    }
}
