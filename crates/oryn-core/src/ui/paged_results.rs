use std::collections::HashSet;

pub struct PagedResults<T> {
    session_id: Option<String>,
    result_count: usize,
    page_size: usize,
    max_pages: usize,
    pages: Vec<(usize, Vec<T>)>,
    in_flight: HashSet<usize>,
}

impl<T> PagedResults<T> {
    pub fn new(page_size: usize, max_pages: usize) -> Self {
        Self {
            session_id: None,
            result_count: 0,
            page_size: page_size.max(1),
            max_pages: max_pages.max(1),
            pages: Vec::new(),
            in_flight: HashSet::new(),
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn result_count(&self) -> usize {
        self.result_count
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn page_offsets(&self) -> Vec<usize> {
        self.pages.iter().map(|(offset, _)| *offset).collect()
    }

    pub fn reset(&mut self, session_id: impl Into<String>, result_count: usize, page_size: usize) {
        self.session_id = Some(session_id.into());
        self.result_count = result_count;
        if page_size > 0 {
            self.page_size = page_size;
        }
        self.pages.clear();
        self.in_flight.clear();
    }

    pub fn clear(&mut self) {
        self.session_id = None;
        self.result_count = 0;
        self.pages.clear();
        self.in_flight.clear();
    }

    fn page_offset(&self, index: usize) -> usize {
        index / self.page_size * self.page_size
    }

    fn touch(&mut self, offset: usize) -> Option<&Vec<T>> {
        let pos = self.pages.iter().position(|(o, _)| *o == offset)?;
        let page = self.pages.remove(pos);
        self.pages.push(page);
        self.pages.last().map(|(_, items)| items)
    }

    pub fn peek(&mut self, index: usize) -> Option<&T> {
        if index >= self.result_count {
            return None;
        }
        let offset = self.page_offset(index);
        self.touch(offset)?.get(index - offset)
    }

    pub fn pages_to_fetch(&mut self, first: usize, last: usize) -> Vec<usize> {
        if self.session_id.is_none() || self.result_count == 0 || first >= self.result_count {
            return Vec::new();
        }
        let end = (self.result_count - 1).min(first.max(last));
        let mut out = Vec::new();
        let mut offset = self.page_offset(first);
        while offset <= end {
            let loaded = self.pages.iter().any(|(o, _)| *o == offset);
            if !loaded && self.in_flight.insert(offset) {
                out.push(offset);
            }
            offset += self.page_size;
        }
        out
    }

    pub fn receive(
        &mut self,
        session_id: &str,
        offset: usize,
        result_count: usize,
        items: Vec<T>,
    ) -> bool {
        if self.session_id.as_deref() != Some(session_id) {
            return false;
        }
        self.in_flight.remove(&offset);
        self.result_count = result_count;
        self.pages.retain(|(o, _)| *o != offset);
        self.pages.push((offset, items));
        while self.pages.len() > self.max_pages {
            self.pages.remove(0);
        }
        true
    }

    pub fn fail(&mut self, session_id: &str, offset: usize) -> bool {
        if self.session_id.as_deref() != Some(session_id) {
            return false;
        }
        self.in_flight.remove(&offset);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(session: &str, offset: usize, size: usize, count: usize) -> Vec<String> {
        (offset..(offset + size).min(count))
            .map(|i| format!("{session}:{i}"))
            .collect()
    }

    fn fetch(store: &mut PagedResults<String>, session: &str, index: usize) -> Vec<usize> {
        let offsets = store.pages_to_fetch(index, index);
        for &offset in &offsets {
            let items = page(session, offset, store.page_size(), 12);
            store.receive(session, offset, 12, items);
        }
        offsets
    }

    #[test]
    fn deduplicates_concurrent_fetches_for_one_page() {
        let mut store = PagedResults::new(2, 6);
        store.reset("session-a", 12, 2);
        assert_eq!(store.pages_to_fetch(0, 1), vec![0]);
        assert!(store.pages_to_fetch(0, 1).is_empty());
        assert!(store.receive("session-a", 0, 12, page("session-a", 0, 2, 12)));
        assert_eq!(store.peek(0).map(String::as_str), Some("session-a:0"));
    }

    #[test]
    fn keeps_only_the_configured_lru_window() {
        let mut store = PagedResults::new(2, 2);
        store.reset("session-a", 12, 2);
        let mut requested = Vec::new();
        for index in [0, 2, 4] {
            requested.extend(fetch(&mut store, "session-a", index));
        }
        assert_eq!(store.page_offsets(), vec![2, 4]);
        store.peek(2);
        requested.extend(fetch(&mut store, "session-a", 6));
        assert_eq!(store.page_offsets(), vec![2, 6]);
        assert_eq!(requested, vec![0, 2, 4, 6]);
    }

    #[test]
    fn drops_a_page_from_a_replaced_session() {
        let mut store = PagedResults::new(2, 6);
        store.reset("old-session", 12, 2);
        assert_eq!(store.pages_to_fetch(0, 1), vec![0]);
        store.reset("new-session", 12, 2);
        assert!(!store.receive("old-session", 0, 12, page("old-session", 0, 2, 12)));
        assert!(store.peek(0).is_none());
        assert_eq!(store.session_id(), Some("new-session"));
    }

    #[test]
    fn a_failed_page_can_be_requested_again() {
        let mut store: PagedResults<String> = PagedResults::new(2, 6);
        store.reset("session-a", 12, 2);
        assert_eq!(store.pages_to_fetch(0, 1), vec![0]);
        assert!(store.fail("session-a", 0));
        assert_eq!(store.pages_to_fetch(0, 1), vec![0]);
    }

    #[test]
    fn out_of_range_requests_do_nothing() {
        let mut store: PagedResults<String> = PagedResults::new(2, 6);
        assert!(store.pages_to_fetch(0, 1).is_empty());
        store.reset("s", 3, 2);
        assert!(store.pages_to_fetch(5, 9).is_empty());
        assert_eq!(store.pages_to_fetch(1, 9), vec![0, 2]);
        assert!(store.peek(3).is_none());
    }
}
