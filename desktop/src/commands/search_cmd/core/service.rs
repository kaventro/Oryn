use super::executor::{ArchiveSearchExecutor, ContentSearchExecutor, NameSearchExecutor};
use super::query::{
    SearchAckOut, SearchCancelIn, SearchPageIn, SearchPageOut, SearchQuery, SearchSessionIn,
    SearchStartIn, SearchStartOut, PAGE_SIZE,
};
use super::session_store::{to_search_results, SearchResultCache, SearchSessionStore};
use super::task_registry::{CancellationSignal, SearchTaskRegistry};
use std::sync::Arc;

#[derive(Default)]
pub struct SearchService {
    cache: SearchResultCache,
    sessions: SearchSessionStore,
    tasks: SearchTaskRegistry,
}

impl SearchService {
    pub async fn start(&self, input: SearchStartIn) -> Result<SearchStartOut, String> {
        let client_id = input.client_id.trim().to_owned();
        if client_id.is_empty() {
            return Err("Search client id is required.".into());
        }
        let query = SearchQuery::try_from(input)?;
        let cancellation = self.tasks.replace(&client_id);
        let result = self.start_query(query, &cancellation).await;
        self.tasks.complete(&client_id, &cancellation);
        result
    }

    async fn start_query(
        &self,
        query: SearchQuery,
        cancellation: &CancellationSignal,
    ) -> Result<SearchStartOut, String> {
        let cache_key = query.cache_key();
        let cache_epoch = self.cache.epoch();

        if cancellation.is_cancelled() {
            return Err("Search cancelled.".into());
        }

        if let Some(results) = self.cache.get(&cache_key) {
            if cancellation.is_cancelled() {
                return Err("Search cancelled.".into());
            }
            let session = self.sessions.open(results);
            return Ok(SearchStartOut {
                ok: true,
                session_id: session.id,
                result_count: session.result_count,
                cached: true,
                page_size: PAGE_SIZE,
            });
        }

        let mut execution = if query.content_text.is_empty() {
            let query_for_worker = query.clone();
            let cancellation_for_worker = cancellation.clone();
            tokio::task::spawn_blocking(move || {
                NameSearchExecutor::execute(query_for_worker, cancellation_for_worker)
            })
            .await
            .map_err(|error| error.to_string())??
        } else {
            ContentSearchExecutor::execute(&query, cancellation).await?
        };

        if cancellation.is_cancelled() {
            return Err("Search cancelled.".into());
        }

        if query.search_in_archives && execution.paths.len() < query.max_results {
            let query_for_worker = query.clone();
            let cancellation_for_worker = cancellation.clone();
            let remaining = query.max_results - execution.paths.len();
            let archive_paths = tokio::task::spawn_blocking(move || {
                ArchiveSearchExecutor::execute(query_for_worker, cancellation_for_worker, remaining)
            })
            .await
            .map_err(|error| error.to_string())??;
            execution.paths.extend(archive_paths);
        }

        if cancellation.is_cancelled() {
            return Err("Search cancelled.".into());
        }

        let results = to_search_results(execution.paths);
        let session = self.sessions.open(Arc::clone(&results));
        self.cache
            .insert_if_current(cache_key, results, cache_epoch);
        Ok(SearchStartOut {
            ok: true,
            session_id: session.id,
            result_count: session.result_count,
            cached: false,
            page_size: PAGE_SIZE,
        })
    }

    pub fn page(&self, input: SearchPageIn) -> Result<SearchPageOut, String> {
        let page = self
            .sessions
            .page(&input.session_id, input.offset, input.limit)?;
        Ok(SearchPageOut {
            ok: true,
            session_id: input.session_id,
            offset: page.offset,
            items: page.items,
            result_count: page.result_count,
        })
    }

    pub fn cancel(&self, input: SearchCancelIn) -> SearchAckOut {
        let client_id = input.client_id.trim();
        if !client_id.is_empty() {
            self.tasks.cancel(client_id);
        }
        SearchAckOut { ok: true }
    }

    pub fn release(&self, input: SearchSessionIn) -> SearchAckOut {
        self.sessions.release(&input.session_id);
        SearchAckOut { ok: true }
    }

    pub fn clear(&self) -> SearchAckOut {
        self.cache.clear();
        SearchAckOut { ok: true }
    }

    pub fn prune_expired(&self) {
        self.cache.prune_expired();
        self.sessions.prune_expired();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::search_cmd::core::query::{EntryTypes, NameMatchMode};
    use std::fs;
    use tempfile::tempdir;

    fn input(root_dir: String) -> SearchStartIn {
        SearchStartIn {
            client_id: "test-client".into(),
            root_dir,
            file_name: "*.rs".into(),
            content_text: String::new(),
            content_not_containing: String::new(),
            exclude_pattern: String::new(),
            max_depth: -1,
            mode: NameMatchMode::Glob,
            entry_types: EntryTypes::Files,
            content_fixed_string: true,
            name_case_sensitive: false,
            content_case_sensitive: false,
            follow_symlinks: false,
            include_hidden: false,
            search_in_archives: false,
            archive_search_contents: false,
            use_native_index: false,
            max_results: 12_000,
        }
    }

    #[tokio::test]
    async fn cache_hit_reuses_results_and_returns_only_a_page() {
        let temp = tempdir().unwrap();
        for index in 0..300 {
            fs::write(temp.path().join(format!("{index}.rs")), "fn main() {}").unwrap();
        }
        let service = SearchService::default();
        let root = temp.path().to_string_lossy().into_owned();

        let first = service.start(input(root.clone())).await.unwrap();
        let page = service
            .page(SearchPageIn {
                session_id: first.session_id.clone(),
                offset: 200,
                limit: 200,
            })
            .unwrap();
        assert_eq!(first.result_count, 300);
        assert_eq!(page.items.len(), 100);

        service.release(SearchSessionIn {
            session_id: first.session_id,
        });
        let second = service.start(input(root)).await.unwrap();
        assert!(second.cached);
        assert_eq!(second.result_count, 300);
        assert_eq!(service.tasks.active_count(), 0);
    }

    #[tokio::test]
    async fn repeated_start_and_release_cycles_leave_no_active_sessions() {
        let temp = tempdir().unwrap();
        for index in 0..300 {
            fs::write(temp.path().join(format!("{index}.rs")), "fn main() {}").unwrap();
        }
        let service = SearchService::default();
        let root = temp.path().to_string_lossy().into_owned();

        for _ in 0..30 {
            let started = service.start(input(root.clone())).await.unwrap();
            let page = service
                .page(SearchPageIn {
                    session_id: started.session_id.clone(),
                    offset: 0,
                    limit: PAGE_SIZE,
                })
                .unwrap();
            assert_eq!(page.items.len(), PAGE_SIZE);
            service.release(SearchSessionIn {
                session_id: started.session_id,
            });
        }

        assert_eq!(service.sessions.active_count(), 0);
    }

    #[tokio::test]
    async fn clearing_the_cache_keeps_an_open_paged_session_readable() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("one.rs"), "fn main() {}").unwrap();
        let service = SearchService::default();
        let started = service
            .start(input(temp.path().to_string_lossy().into_owned()))
            .await
            .unwrap();

        service.clear();
        let page = service
            .page(SearchPageIn {
                session_id: started.session_id,
                offset: 0,
                limit: PAGE_SIZE,
            })
            .unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn twelve_thousand_result_search_keeps_ipc_pages_at_two_hundred_items() {
        let temp = tempdir().unwrap();
        for index in 0..12_000 {
            fs::write(temp.path().join(format!("hit-{index:05}.txt")), "fixture").unwrap();
        }
        let service = SearchService::default();
        let mut request = input(temp.path().to_string_lossy().into_owned());
        request.file_name = "hit-".into();
        request.mode = NameMatchMode::Substring;

        let started = service.start(request).await.unwrap();
        assert_eq!(started.result_count, 12_000);
        let first_page = service
            .page(SearchPageIn {
                session_id: started.session_id,
                offset: 0,
                limit: 10_000,
            })
            .unwrap();
        assert_eq!(first_page.items.len(), PAGE_SIZE);
    }
}
