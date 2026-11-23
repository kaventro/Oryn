use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::*;
use oryn_core::events::EventSink;
use oryn_core::services::fs_watcher::FsWatcherService;
use parking_lot::Mutex;

use super::Workspace;

struct Collect(Arc<Mutex<HashSet<PathBuf>>>);

impl EventSink for Collect {
    fn emit(&self, _: &str, payload: serde_json::Value) {
        if let Some(path) = payload.get("path").and_then(|p| p.as_str()) {
            self.0.lock().insert(PathBuf::from(path));
        }
    }
}

pub struct Watch {
    service: FsWatcherService,
    dirty: Arc<Mutex<HashSet<PathBuf>>>,
    watched: Vec<PathBuf>,
    _task: Task<()>,
}

impl Workspace {
    pub(super) fn start_watch(cx: &mut Context<Self>) -> Watch {
        let dirty = Arc::new(Mutex::new(HashSet::new()));
        let service = FsWatcherService::default();
        service.set_sink(Arc::new(Collect(dirty.clone())));
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
                if this.update(cx, |ws, cx| ws.poll_watch(cx)).is_err() {
                    break;
                }
            }
        });
        Watch {
            service,
            dirty,
            watched: Vec::new(),
            _task: task,
        }
    }

    fn poll_watch(&mut self, cx: &mut Context<Self>) {
        let mut dirs: Vec<PathBuf> = self
            .tabs
            .iter()
            .flatten()
            .map(|p| p.read(cx).path.clone())
            .collect();
        dirs.sort();
        dirs.dedup();
        let Some(watch) = &mut self.watch else {
            return;
        };
        if dirs != watch.watched {
            let _ = watch.service.watch_dirs(
                dirs.iter()
                    .map(|d| d.to_string_lossy().into_owned())
                    .collect(),
            );
            watch.watched = dirs;
            watch.dirty.lock().clear();
            return;
        }
        let changed: HashSet<PathBuf> = std::mem::take(&mut *watch.dirty.lock());
        if changed.is_empty() || self.job.is_some() {
            if self.job.is_some() {
                watch.dirty.lock().extend(changed);
            }
            return;
        }
        for pane in self.tabs.iter().flatten().cloned().collect::<Vec<_>>() {
            if changed.contains(&pane.read(cx).path) {
                pane.update(cx, |p, cx| p.refresh(None, cx));
            }
        }
    }
}
