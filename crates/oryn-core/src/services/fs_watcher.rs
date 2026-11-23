use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::events::EventSink;

#[derive(Clone, Serialize)]
pub struct FsChangeEvent {
    pub path: String,
}

pub struct FsWatcherService {
    watcher: Mutex<Option<RecommendedWatcher>>,
    watched_dirs: Arc<Mutex<HashSet<PathBuf>>>,
    sink: Mutex<Option<Arc<dyn EventSink>>>,
}

impl Default for FsWatcherService {
    fn default() -> Self {
        Self {
            watcher: Mutex::new(None),
            watched_dirs: Arc::new(Mutex::new(HashSet::new())),
            sink: Mutex::new(None),
        }
    }
}

impl FsWatcherService {
    pub fn set_sink(&self, sink: Arc<dyn EventSink>) {
        *self.sink.lock() = Some(sink);
    }

    pub fn watch_dirs(&self, paths: Vec<String>) -> Result<(), String> {
        let mut watcher_lock = self.watcher.lock();
        let mut watched_dirs_lock = self.watched_dirs.lock();

        *watcher_lock = None;
        watched_dirs_lock.clear();

        if paths.is_empty() {
            return Ok(());
        }

        let sink_opt = self.sink.lock().clone();
        let watched_dirs_clone = Arc::clone(&self.watched_dirs);

        let event_handler = move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if let Some(sink) = &sink_opt {
                    let watched = watched_dirs_clone.lock();
                    let mut affected_dirs = HashSet::new();

                    for event_path in &event.paths {
                        if watched.contains(event_path) {
                            affected_dirs.insert(event_path.clone());
                        }
                        if let Some(parent) = event_path.parent() {
                            if watched.contains(parent) {
                                affected_dirs.insert(parent.to_path_buf());
                            }
                        }
                    }

                    for dir in affected_dirs {
                        let path_str = dir.to_string_lossy().to_string();
                        sink.emit(
                            "fs:change",
                            serde_json::to_value(FsChangeEvent { path: path_str })
                                .unwrap_or_default(),
                        );
                    }
                }
            }
        };

        let mut watcher = RecommendedWatcher::new(event_handler, Config::default())
            .map_err(|e| format!("Failed to create watcher: {e}"))?;

        for p_str in paths {
            let p = PathBuf::from(&p_str);
            if p.exists() && p.is_dir() {
                if let Err(e) = watcher.watch(Path::new(&p), RecursiveMode::NonRecursive) {
                    tracing::warn!("Failed to watch path {:?}: {}", p, e);
                } else {
                    watched_dirs_lock.insert(p);
                }
            }
        }

        *watcher_lock = Some(watcher);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_empty_dirs_clears_state() {
        let service = FsWatcherService::default();
        let r = service.watch_dirs(vec![]);
        assert!(r.is_ok());
        assert!(service.watched_dirs.lock().is_empty());
    }

    #[test]
    fn watch_valid_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let service = FsWatcherService::default();
        let r = service.watch_dirs(vec![path.clone()]);
        assert!(r.is_ok());
        assert!(service.watched_dirs.lock().contains(&PathBuf::from(&path)));
    }

    #[test]
    fn watch_nonexistent_directory_is_skipped_without_error() {
        let service = FsWatcherService::default();
        let r = service.watch_dirs(vec!["/nonexistent_path_xyz123".to_string()]);
        assert!(r.is_ok());
        assert!(service.watched_dirs.lock().is_empty());
    }
}
