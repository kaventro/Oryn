use parking_lot::Mutex;
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type SearchResults = Arc<[String]>;

const CACHE_TTL: Duration = Duration::from_secs(90);
const CACHE_MAX_ENTRIES: usize = 4;
const CACHE_MAX_BYTES: usize = 4 * 1024 * 1024;
const SESSION_TTL: Duration = Duration::from_secs(120);
const MAX_ACTIVE_SESSIONS: usize = 2;
const MAX_PAGE_SIZE: usize = 200;

#[derive(Default)]
pub struct SearchResultCache {
    inner: Mutex<CacheState>,
}

#[derive(Default)]
struct CacheState {
    entries: HashMap<String, CachedResult>,
    bytes: usize,
    epoch: u64,
}

struct CachedResult {
    results: SearchResults,
    touched_at: Instant,
    bytes: usize,
}

impl SearchResultCache {
    pub fn get(&self, key: &str) -> Option<SearchResults> {
        let mut state = self.inner.lock();
        state.remove_expired(Instant::now());
        let entry = state.entries.get_mut(key)?;
        entry.touched_at = Instant::now();
        Some(Arc::clone(&entry.results))
    }

    pub fn epoch(&self) -> u64 {
        self.inner.lock().epoch
    }

    pub fn insert_if_current(&self, key: String, results: SearchResults, epoch: u64) {
        let bytes = estimated_result_bytes(&key, &results);
        if bytes > CACHE_MAX_BYTES {
            return;
        }

        let mut state = self.inner.lock();
        if state.epoch != epoch {
            return;
        }
        state.remove_expired(Instant::now());
        state.remove(&key);
        while state.entries.len() >= CACHE_MAX_ENTRIES || state.bytes + bytes > CACHE_MAX_BYTES {
            state.evict_oldest();
        }
        state.bytes += bytes;
        state.entries.insert(
            key,
            CachedResult {
                results,
                touched_at: Instant::now(),
                bytes,
            },
        );
    }

    pub fn clear(&self) {
        let mut state = self.inner.lock();
        state.entries.clear();
        state.bytes = 0;
        state.epoch = state.epoch.wrapping_add(1);
    }

    pub fn prune_expired(&self) {
        self.inner.lock().remove_expired(Instant::now());
    }
}

impl CacheState {
    fn remove_expired(&mut self, now: Instant) {
        let expired: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.touched_at) >= CACHE_TTL)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            self.remove(&key);
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
    }

    fn evict_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.touched_at)
            .map(|(key, _)| key.clone());
        if let Some(key) = oldest {
            self.remove(&key);
        }
    }
}


#[derive(Default)]
pub struct SearchSessionStore {
    inner: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    next_id: u64,
    next_touch: u64,
    entries: HashMap<String, SearchSession>,
}

struct SearchSession {
    results: SearchResults,
    touched_at: Instant,
    last_used: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchSessionSummary {
    pub id: String,
    pub result_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPage {
    pub offset: usize,
    pub items: Vec<String>,
    pub result_count: usize,
}

impl SearchSessionStore {
    pub fn open(&self, results: SearchResults) -> SearchSessionSummary {
        let mut state = self.inner.lock();
        state.remove_expired(Instant::now());
        while state.entries.len() >= MAX_ACTIVE_SESSIONS {
            state.evict_oldest();
        }
        state.next_id = state.next_id.wrapping_add(1);
        let last_used = state.next_touch();
        let id = format!("search-{:016x}", state.next_id);
        let result_count = results.len();
        state.entries.insert(
            id.clone(),
            SearchSession {
                results,
                touched_at: Instant::now(),
                last_used,
            },
        );
        SearchSessionSummary { id, result_count }
    }

    pub fn page(
        &self,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<SearchPage, String> {
        let mut state = self.inner.lock();
        state.remove_expired(Instant::now());
        let last_used = state.next_touch();
        let session = state
            .entries
            .get_mut(session_id)
            .ok_or_else(|| "Search session expired. Run the search again.".to_string())?;
        session.touched_at = Instant::now();
        session.last_used = last_used;

        let result_count = session.results.len();
        let offset = offset.min(result_count);
        let end = offset
            .saturating_add(limit.clamp(1, MAX_PAGE_SIZE))
            .min(result_count);
        Ok(SearchPage {
            offset,
            items: session.results[offset..end].to_vec(),
            result_count,
        })
    }

    pub fn release(&self, session_id: &str) {
        self.inner.lock().entries.remove(session_id);
    }

    pub fn prune_expired(&self) {
        self.inner.lock().remove_expired(Instant::now());
    }

    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.inner.lock().entries.len()
    }
}

impl SessionState {
    fn next_touch(&mut self) -> u64 {
        self.next_touch = self.next_touch.wrapping_add(1);
        self.next_touch
    }

    fn remove_expired(&mut self, now: Instant) {
        self.entries
            .retain(|_, session| now.duration_since(session.touched_at) < SESSION_TTL);
    }

    fn evict_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, session)| session.last_used)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest {
            self.entries.remove(&id);
        }
    }
}

pub fn to_search_results(paths: Vec<String>) -> SearchResults {
    Arc::from(paths.into_boxed_slice())
}

fn estimated_result_bytes(key: &str, results: &SearchResults) -> usize {
    key.len()
        + results.len() * size_of::<String>()
        + results.iter().map(String::capacity).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_bounded_and_keep_the_correct_offset() {
        let paths = (0..450).map(|n| format!("/tmp/{n}")).collect();
        let store = SearchSessionStore::default();
        let session = store.open(to_search_results(paths));

        let page = store.page(&session.id, 200, 10_000).unwrap();
        assert_eq!(page.offset, 200);
        assert_eq!(page.items.len(), MAX_PAGE_SIZE);
        assert_eq!(page.items.first().unwrap(), "/tmp/200");
        assert_eq!(page.result_count, 450);
    }

    #[test]
    fn cache_returns_the_same_arc_without_copying_all_results() {
        let cache = SearchResultCache::default();
        let results = to_search_results(vec!["/tmp/a".into(), "/tmp/b".into()]);
        cache.insert_if_current("query".into(), Arc::clone(&results), cache.epoch());

        let cached = cache.get("query").unwrap();
        assert!(Arc::ptr_eq(&results, &cached));
    }

    #[test]
    fn cache_clear_rejects_a_result_from_an_older_search_epoch() {
        let cache = SearchResultCache::default();
        let stale_epoch = cache.epoch();
        cache.clear();
        cache.insert_if_current(
            "query".into(),
            to_search_results(vec!["/tmp/stale".into()]),
            stale_epoch,
        );

        assert!(cache.get("query").is_none());
    }

    #[test]
    fn released_session_cannot_be_read_again() {
        let store = SearchSessionStore::default();
        let session = store.open(to_search_results(vec!["/tmp/a".into()]));
        store.release(&session.id);

        assert!(store.page(&session.id, 0, 1).is_err());
    }

    #[test]
    fn opening_a_new_session_evicts_the_least_recently_used_one_at_capacity() {
        let store = SearchSessionStore::default();
        let first = store.open(to_search_results(vec!["/tmp/a".into()]));
        let second = store.open(to_search_results(vec!["/tmp/b".into()]));
        let third = store.open(to_search_results(vec!["/tmp/c".into()]));

        assert!(store.page(&first.id, 0, 1).is_err());
        assert!(store.page(&second.id, 0, 1).is_ok());
        assert!(store.page(&third.id, 0, 1).is_ok());
    }
}
