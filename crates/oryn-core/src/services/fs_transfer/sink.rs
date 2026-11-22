use serde_json::Value;

pub trait ProgressSink: Send + Sync {
    fn emit(&self, payload: Value);
    fn is_aborted(&self) -> bool;
}
