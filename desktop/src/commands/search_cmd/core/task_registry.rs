use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::watch;


#[derive(Clone)]
pub struct CancellationSignal {
    state: Arc<CancellationState>,
}

struct CancellationState {
    cancelled: AtomicBool,
    changed: watch::Sender<bool>,
}

impl CancellationSignal {
    pub fn new() -> Self {
        let (changed, _) = watch::channel(false);
        Self {
            state: Arc::new(CancellationState {
                cancelled: AtomicBool::new(false),
                changed,
            }),
        }
    }

    pub fn cancel(&self) {
        if !self.state.cancelled.swap(true, Ordering::AcqRel) {
            self.state.changed.send_replace(true);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }

    pub async fn cancelled(&self) {
        let mut receiver = self.state.changed.subscribe();
        if *receiver.borrow() {
            return;
        }
        let _ = receiver.changed().await;
    }

    fn is_same_task(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
}


#[derive(Default)]
pub struct SearchTaskRegistry {
    tasks: Mutex<HashMap<String, CancellationSignal>>,
}

impl SearchTaskRegistry {
    pub fn replace(&self, client_id: &str) -> CancellationSignal {
        let next = CancellationSignal::new();
        let previous = self.tasks.lock().insert(client_id.to_owned(), next.clone());
        if let Some(previous) = previous {
            previous.cancel();
        }
        next
    }

    pub fn cancel(&self, client_id: &str) {
        if let Some(task) = self.tasks.lock().get(client_id).cloned() {
            task.cancel();
        }
    }

    pub fn complete(&self, client_id: &str, task: &CancellationSignal) {
        let mut tasks = self.tasks.lock();
        if tasks
            .get(client_id)
            .is_some_and(|current| current.is_same_task(task))
        {
            tasks.remove(client_id);
        }
    }

    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.tasks.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replacing_one_client_cancels_only_its_previous_task() {
        let registry = SearchTaskRegistry::default();
        let first = registry.replace("window-a");
        let other_window = registry.replace("window-b");
        let replacement = registry.replace("window-a");

        assert!(first.is_cancelled());
        assert!(!replacement.is_cancelled());
        assert!(!other_window.is_cancelled());

        registry.complete("window-a", &first);
        assert_eq!(registry.active_count(), 2);
        registry.complete("window-a", &replacement);
        assert_eq!(registry.active_count(), 1);
    }
}
