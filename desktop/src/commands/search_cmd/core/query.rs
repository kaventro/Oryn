use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_RESULTS: usize = 12_000;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStartIn {
    #[serde(default)]
    pub client_id: String,
    pub root_dir: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub content_text: String,
    #[serde(default)]
    pub content_not_containing: String,
    #[serde(default)]
    pub exclude_pattern: String,
    #[serde(default = "unlimited_depth")]
    pub max_depth: i32,
    #[serde(default)]
    pub mode: NameMatchMode,
    #[serde(default)]
    pub entry_types: EntryTypes,
    #[serde(default = "default_true", alias = "rgFixedString")]
    pub content_fixed_string: bool,
    #[serde(default)]
    pub name_case_sensitive: bool,
    #[serde(default)]
    pub content_case_sensitive: bool,
    #[serde(default)]
    pub follow_symlinks: bool,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default, alias = "searchInZips")]
    pub search_in_archives: bool,
    #[serde(default, alias = "contentInZips")]
    pub archive_search_contents: bool,
    #[serde(default)]
    pub use_native_index: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NameMatchMode {
    Exact,
    #[default]
    Substring,
    Glob,
    Regex,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryTypes {
    #[default]
    All,
    Files,
    Dirs,
}

#[derive(Clone, Debug)]
pub struct SearchQuery {
    pub root_dir: PathBuf,
    pub file_name: String,
    pub content_text: String,
    pub content_not_containing: String,
    pub exclude_pattern: String,
    pub max_depth: Option<usize>,
    pub mode: NameMatchMode,
    pub entry_types: EntryTypes,
    pub content_fixed_string: bool,
    pub name_case_sensitive: bool,
    pub content_case_sensitive: bool,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub search_in_archives: bool,
    pub archive_search_contents: bool,
    pub max_results: usize,
}

impl TryFrom<SearchStartIn> for SearchQuery {
    type Error = String;

    fn try_from(input: SearchStartIn) -> Result<Self, Self::Error> {
        // Keep the existing UI option wire-compatible until a native index
        // strategy is implemented. The normal filesystem strategy is the
        // deliberate fallback for now.
        let _native_index_fallback = input.use_native_index;
        let root_dir = PathBuf::from(input.root_dir.trim());
        if !root_dir.is_dir() {
            return Err("Search root must be an existing directory.".into());
        }
        if input.max_depth < -1 {
            return Err("Search depth must be -1 or a non-negative number.".into());
        }

        let file_name = input.file_name.trim().to_string();
        let content_text = input.content_text.trim().to_string();
        let content_not_containing = input.content_not_containing.trim().to_string();
        if file_name.is_empty() && content_text.is_empty() {
            return Err("Enter a file name or text to search for.".into());
        }
        if !content_not_containing.is_empty() && content_text.is_empty() {
            return Err("“Does NOT contain” requires a positive content query.".into());
        }

        Ok(Self {
            root_dir,
            file_name,
            content_text,
            content_not_containing,
            exclude_pattern: input.exclude_pattern.trim().to_string(),
            max_depth: (input.max_depth >= 0).then_some(input.max_depth as usize),
            mode: input.mode,
            entry_types: input.entry_types,
            content_fixed_string: input.content_fixed_string,
            name_case_sensitive: input.name_case_sensitive,
            content_case_sensitive: input.content_case_sensitive,
            follow_symlinks: input.follow_symlinks,
            include_hidden: input.include_hidden,
            search_in_archives: input.search_in_archives,
            archive_search_contents: input.archive_search_contents,
            max_results: input.max_results.clamp(1, MAX_RESULTS),
        })
    }
}

impl SearchQuery {
    pub fn cache_key(&self) -> String {
        serde_json::to_string(&SearchQueryKey::from(self)).unwrap_or_default()
    }

    pub fn name_parts(&self) -> Vec<String> {
        self.file_name
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub fn exclude_parts(&self) -> Vec<String> {
        self.exclude_pattern
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchQueryKey<'a> {
    root_dir: String,
    file_name: &'a str,
    content_text: &'a str,
    content_not_containing: &'a str,
    exclude_pattern: &'a str,
    max_depth: Option<usize>,
    mode: NameMatchMode,
    entry_types: EntryTypes,
    content_fixed_string: bool,
    name_case_sensitive: bool,
    content_case_sensitive: bool,
    follow_symlinks: bool,
    include_hidden: bool,
    search_in_archives: bool,
    archive_search_contents: bool,
    max_results: usize,
}

impl<'a> From<&'a SearchQuery> for SearchQueryKey<'a> {
    fn from(query: &'a SearchQuery) -> Self {
        Self {
            root_dir: query.root_dir.to_string_lossy().into_owned(),
            file_name: &query.file_name,
            content_text: &query.content_text,
            content_not_containing: &query.content_not_containing,
            exclude_pattern: &query.exclude_pattern,
            max_depth: query.max_depth,
            mode: query.mode,
            entry_types: query.entry_types,
            content_fixed_string: query.content_fixed_string,
            name_case_sensitive: query.name_case_sensitive,
            content_case_sensitive: query.content_case_sensitive,
            follow_symlinks: query.follow_symlinks,
            include_hidden: query.include_hidden,
            search_in_archives: query.search_in_archives,
            archive_search_contents: query.archive_search_contents,
            max_results: query.max_results,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPageIn {
    pub session_id: String,
    pub offset: usize,
    #[serde(default = "default_page_size")]
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSessionIn {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCancelIn {
    pub client_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStartOut {
    pub ok: bool,
    pub session_id: String,
    pub result_count: usize,
    pub cached: bool,
    pub page_size: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPageOut {
    pub ok: bool,
    pub session_id: String,
    pub offset: usize,
    pub items: Vec<String>,
    pub result_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchAckOut {
    pub ok: bool,
}

pub const PAGE_SIZE: usize = 200;

fn unlimited_depth() -> i32 {
    -1
}

fn default_max_results() -> usize {
    MAX_RESULTS
}

fn default_page_size() -> usize {
    PAGE_SIZE
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_all_supported_filters_into_a_canonical_key() {
        let temp = tempdir().unwrap();
        let query = SearchQuery::try_from(SearchStartIn {
            client_id: "test-client".into(),
            root_dir: temp.path().to_string_lossy().into_owned(),
            file_name: "*.rs".into(),
            content_text: "SearchService".into(),
            content_not_containing: "todo".into(),
            exclude_pattern: "target;node_modules".into(),
            max_depth: 2,
            mode: NameMatchMode::Glob,
            entry_types: EntryTypes::Files,
            content_fixed_string: true,
            name_case_sensitive: false,
            content_case_sensitive: true,
            follow_symlinks: false,
            include_hidden: false,
            search_in_archives: false,
            archive_search_contents: false,
            // Spotlight is an optional future strategy; while it is not
            // available, the request intentionally falls back to normal
            // filesystem search instead of failing the visible UI control.
            use_native_index: true,
            max_results: 99_999,
        })
        .unwrap();

        assert_eq!(query.max_depth, Some(2));
        assert_eq!(query.max_results, MAX_RESULTS);
        assert_eq!(query.exclude_parts(), vec!["target", "node_modules"]);
        assert!(!query.cache_key().is_empty());
    }
}
