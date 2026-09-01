use super::matcher;
use super::query::{EntryTypes, NameMatchMode, SearchQuery};
use super::task_registry::CancellationSignal;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use walkdir::{DirEntry, WalkDir};
use zip::ZipArchive;

const CONTENT_EXCLUSION_CHUNK_SIZE: usize = 128;
const MAX_ARCHIVE_ENTRY_CONTENT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct SearchExecution {
    pub paths: Vec<String>,
}

pub struct NameSearchExecutor;

impl NameSearchExecutor {
    pub fn execute(
        query: SearchQuery,
        cancellation: CancellationSignal,
    ) -> Result<SearchExecution, String> {
        let parts = query.name_parts();
        let matcher = matcher::build_name_matcher(
            &parts,
            match query.mode {
                NameMatchMode::Exact => "exact",
                NameMatchMode::Substring => "substring",
                NameMatchMode::Glob => "glob",
                NameMatchMode::Regex => "regex",
            },
            query.name_case_sensitive,
        )?;
        let excludes = ExcludeMatcher::new(&query.exclude_parts())?;
        let traversal_excludes = excludes.clone();
        let include_hidden = query.include_hidden;
        let mut walker = WalkDir::new(&query.root_dir)
            .min_depth(1)
            .follow_links(query.follow_symlinks);
        if let Some(depth) = query.max_depth {
            walker = walker.max_depth(depth.saturating_add(1));
        }
        let mut paths = Vec::new();
        let iterator = walker.into_iter().filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            if !include_hidden && is_hidden(entry) {
                return false;
            }
            let file_name = entry.file_name().to_string_lossy();
            !traversal_excludes.matches(entry.path(), &file_name)
        });
        for entry in iterator {
            if cancellation.is_cancelled() {
                return Err("Search cancelled.".into());
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let file_name = entry.file_name().to_string_lossy();
            let is_dir = entry.file_type().is_dir();
            if query.entry_types == EntryTypes::Files && is_dir {
                continue;
            }
            if query.entry_types == EntryTypes::Dirs && !is_dir {
                continue;
            }
            if !matcher(&file_name, is_dir) {
                continue;
            }
            paths.push(entry.path().to_string_lossy().into_owned());
            if paths.len() >= query.max_results {
                return Ok(SearchExecution { paths });
            }
        }
        Ok(SearchExecution { paths })
    }
}

pub struct ContentSearchExecutor;

impl ContentSearchExecutor {
    pub async fn execute(
        query: &SearchQuery,
        cancellation: &CancellationSignal,
    ) -> Result<SearchExecution, String> {
        if query.entry_types == EntryTypes::Dirs {
            return Err(
                "Text search applies to file contents — choose files or files + folders.".into(),
            );
        }

        let name_filter = Self::name_filter(query)?;
        let mut candidates = PathStream::spawn(Self::command_for_root(query, &query.content_text))?;
        let paths = if query.content_not_containing.is_empty() {
            Self::collect_matching_paths(
                &mut candidates,
                name_filter.as_ref(),
                query.max_results,
                cancellation,
            )
            .await?
        } else {
            Self::collect_without_prohibited(
                query,
                &mut candidates,
                name_filter.as_ref(),
                cancellation,
            )
            .await?
        };
        Ok(SearchExecution { paths })
    }

    fn name_filter(query: &SearchQuery) -> Result<Option<matcher::NameMatcher>, String> {
        let parts = query.name_parts();
        if parts.is_empty() {
            return Ok(None);
        }
        matcher::build_name_matcher(
            &parts,
            match query.mode {
                NameMatchMode::Exact => "exact",
                NameMatchMode::Substring => "substring",
                NameMatchMode::Glob => "glob",
                NameMatchMode::Regex => "regex",
            },
            query.name_case_sensitive,
        )
        .map(Some)
    }

    fn command_for_root(query: &SearchQuery, content: &str) -> Command {
        let mut command = Self::base_command(query, content);
        command.arg("--").arg(&query.root_dir);
        command
    }

    async fn collect_matching_paths(
        stream: &mut PathStream,
        name_filter: Option<&matcher::NameMatcher>,
        limit: usize,
        cancellation: &CancellationSignal,
    ) -> Result<Vec<String>, String> {
        let mut paths = Vec::new();
        while paths.len() < limit {
            let Some(path) = Self::next_matching_path(stream, name_filter, cancellation).await?
            else {
                break;
            };
            paths.push(path);
        }
        if paths.len() == limit {
            stream.stop().await;
        }
        Ok(paths)
    }

    async fn collect_without_prohibited(
        query: &SearchQuery,
        stream: &mut PathStream,
        name_filter: Option<&matcher::NameMatcher>,
        cancellation: &CancellationSignal,
    ) -> Result<Vec<String>, String> {
        let mut accepted = Vec::new();
        loop {
            let mut candidates = Vec::with_capacity(CONTENT_EXCLUSION_CHUNK_SIZE);
            while candidates.len() < CONTENT_EXCLUSION_CHUNK_SIZE {
                let Some(path) =
                    Self::next_matching_path(stream, name_filter, cancellation).await?
                else {
                    break;
                };
                candidates.push(path);
            }
            if candidates.is_empty() {
                break;
            }

            let prohibited = Self::find_prohibited_paths(query, &candidates, cancellation).await?;
            for path in candidates {
                if !prohibited.contains(&path) {
                    accepted.push(path);
                    if accepted.len() == query.max_results {
                        stream.stop().await;
                        return Ok(accepted);
                    }
                }
            }
        }
        Ok(accepted)
    }

    async fn next_matching_path(
        stream: &mut PathStream,
        name_filter: Option<&matcher::NameMatcher>,
        cancellation: &CancellationSignal,
    ) -> Result<Option<String>, String> {
        while let Some(path) = stream.next_path(cancellation).await? {
            let matches_name = name_filter.is_none_or(|matches| {
                let file_name = Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_default();
                matches(&file_name, false)
            });
            if matches_name {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    async fn find_prohibited_paths(
        query: &SearchQuery,
        candidates: &[String],
        cancellation: &CancellationSignal,
    ) -> Result<HashSet<String>, String> {
        if cancellation.is_cancelled() {
            return Err("Search cancelled.".into());
        }
        let mut command = Self::base_command(query, &query.content_not_containing);
        command.arg("--").args(candidates);
        let mut stream = PathStream::spawn(command)?;
        let paths =
            Self::collect_matching_paths(&mut stream, None, candidates.len(), cancellation).await?;
        Ok(paths.into_iter().collect())
    }

    fn base_command(query: &SearchQuery, content: &str) -> Command {
        let mut command = Command::new("rg");
        command.args(["-l", "--null"]);
        command.kill_on_drop(true);
        command.stdout(Stdio::piped()).stderr(Stdio::null());
        if query.follow_symlinks {
            command.arg("--follow");
        }
        if query.include_hidden {
            command.arg("--hidden");
        }
        if let Some(depth) = query.max_depth {
            let depth = depth.saturating_add(1).to_string();
            command.args(["--max-depth", depth.as_str()]);
        }
        for pattern in [
            ".git/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/.npm/**".to_string(),
            "**/.Trash/**".to_string(),
        ]
        .into_iter()
        .chain(query.exclude_parts().into_iter().flat_map(exclude_globs))
        {
            command.args(["--glob", &format!("!{pattern}")]);
        }
        if query.content_fixed_string {
            command.arg("-F");
        }
        if !query.content_case_sensitive {
            command.arg("-i");
        }
        command.arg("-e").arg(content);
        command
    }
}

pub struct ArchiveSearchExecutor;

impl ArchiveSearchExecutor {
    pub fn execute(
        query: SearchQuery,
        cancellation: CancellationSignal,
        limit: usize,
    ) -> Result<Vec<String>, String> {
        if limit == 0 || !query.search_in_archives {
            return Ok(Vec::new());
        }
        let name_filter = Self::name_filter(&query)?;
        let content_filter = TextMatcher::new(
            &query.content_text,
            query.content_fixed_string,
            query.content_case_sensitive,
        )?;
        let prohibited_filter = TextMatcher::new(
            &query.content_not_containing,
            query.content_fixed_string,
            query.content_case_sensitive,
        )?;
        let needs_content = content_filter.is_some() && query.archive_search_contents;
        if name_filter.is_none() && !needs_content {
            return Ok(Vec::new());
        }

        let excludes = ExcludeMatcher::new(&query.exclude_parts())?;
        let traversal_excludes = excludes.clone();
        let include_hidden = query.include_hidden;
        let mut walker = WalkDir::new(&query.root_dir)
            .min_depth(1)
            .follow_links(query.follow_symlinks);
        if let Some(depth) = query.max_depth {
            walker = walker.max_depth(depth.saturating_add(1));
        }

        let mut hits = Vec::new();
        let iterator = walker.into_iter().filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            if !include_hidden && is_hidden(entry) {
                return false;
            }
            let file_name = entry.file_name().to_string_lossy();
            !traversal_excludes.matches(entry.path(), &file_name)
        });
        for entry in iterator {
            if cancellation.is_cancelled() {
                return Err("Search cancelled.".into());
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() || !is_supported_archive(entry.path()) {
                continue;
            }
            let archive_path = entry.path().to_string_lossy().into_owned();
            Self::search_archive(
                &archive_path,
                name_filter.as_ref(),
                content_filter.as_ref().filter(|_| needs_content),
                prohibited_filter.as_ref().filter(|_| needs_content),
                &excludes,
                query.entry_types,
                &cancellation,
                limit,
                &mut hits,
            )?;
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    fn name_filter(query: &SearchQuery) -> Result<Option<matcher::NameMatcher>, String> {
        let parts = query.name_parts();
        if parts.is_empty() {
            return Ok(None);
        }
        matcher::build_name_matcher(
            &parts,
            match query.mode {
                NameMatchMode::Exact => "exact",
                NameMatchMode::Substring => "substring",
                NameMatchMode::Glob => "glob",
                NameMatchMode::Regex => "regex",
            },
            query.name_case_sensitive,
        )
        .map(Some)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_archive(
        archive_path: &str,
        name_filter: Option<&matcher::NameMatcher>,
        content_filter: Option<&TextMatcher>,
        prohibited_filter: Option<&TextMatcher>,
        excludes: &ExcludeMatcher,
        entry_types: EntryTypes,
        cancellation: &CancellationSignal,
        limit: usize,
        hits: &mut Vec<String>,
    ) -> Result<(), String> {
        let file = match File::open(archive_path) {
            Ok(file) => file,
            Err(_) => return Ok(()),
        };
        let mut archive = match ZipArchive::new(file) {
            Ok(archive) => archive,
            Err(_) => return Ok(()),
        };
        for index in 0..archive.len() {
            if cancellation.is_cancelled() {
                return Err("Search cancelled.".into());
            }
            if hits.len() >= limit {
                return Ok(());
            }
            let mut entry = match archive.by_index(index) {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let is_dir = entry.is_dir();
            if entry_types == EntryTypes::Files && is_dir {
                continue;
            }
            if entry_types == EntryTypes::Dirs && !is_dir {
                continue;
            }
            let entry_name = entry.name().to_owned();
            let base_name = Path::new(&entry_name)
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();
            if excludes.matches(Path::new(&entry_name), &base_name) {
                continue;
            }
            if !name_filter.is_none_or(|matches| matches(&base_name, is_dir)) {
                continue;
            }
            if let Some(content_filter) = content_filter {
                if is_dir || entry.size() > MAX_ARCHIVE_ENTRY_CONTENT_BYTES {
                    continue;
                }
                // Bound the ACTUAL decompressed read: a malicious archive can
                // declare a small uncompressed size (passing the check above)
                // while its deflate stream expands to gigabytes. Cap the read at
                // the limit and skip entries whose real content exceeds it.
                let cap = (entry.size() as usize).min(MAX_ARCHIVE_ENTRY_CONTENT_BYTES as usize);
                let mut bytes = Vec::with_capacity(cap);
                if entry
                    .by_ref()
                    .take(MAX_ARCHIVE_ENTRY_CONTENT_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .is_err()
                {
                    continue;
                }
                if bytes.len() as u64 > MAX_ARCHIVE_ENTRY_CONTENT_BYTES {
                    continue;
                }
                let text = String::from_utf8_lossy(&bytes);
                if !content_filter.is_match(&text)
                    || prohibited_filter.is_some_and(|prohibited| prohibited.is_match(&text))
                {
                    continue;
                }
            }
            hits.push(format!("{archive_path}#{entry_name}"));
        }
        Ok(())
    }
}

enum TextMatcher {
    Literal {
        needle: String,
        case_sensitive: bool,
    },
    Regex(regex::Regex),
}

impl TextMatcher {
    fn new(
        pattern: &str,
        fixed_string: bool,
        case_sensitive: bool,
    ) -> Result<Option<Self>, String> {
        if pattern.is_empty() {
            return Ok(None);
        }
        if fixed_string {
            return Ok(Some(Self::Literal {
                needle: pattern.to_owned(),
                case_sensitive,
            }));
        }
        regex::RegexBuilder::new(pattern)
            .case_insensitive(!case_sensitive)
            .build()
            .map(Self::Regex)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn is_match(&self, text: &str) -> bool {
        match self {
            Self::Literal {
                needle,
                case_sensitive,
            } => {
                if *case_sensitive {
                    text.contains(needle)
                } else {
                    text.to_lowercase().contains(&needle.to_lowercase())
                }
            }
            Self::Regex(regex) => regex.is_match(text),
        }
    }
}

fn is_supported_archive(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("zip" | "jar" | "war" | "ear")
    )
}

struct PathStream {
    child: Child,
    output: BufReader<ChildStdout>,
    buffer: Vec<u8>,
    finished: bool,
}

impl PathStream {
    fn spawn(mut command: Command) -> Result<Self, String> {
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "ripgrep (rg) is not installed or available on PATH.".to_string()
            } else {
                error.to_string()
            }
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Could not read ripgrep output.".to_string())?;
        Ok(Self {
            child,
            output: BufReader::new(stdout),
            buffer: Vec::new(),
            finished: false,
        })
    }

    async fn next_path(
        &mut self,
        cancellation: &CancellationSignal,
    ) -> Result<Option<String>, String> {
        if self.finished {
            return Ok(None);
        }
        self.buffer.clear();
        let read = {
            let output = &mut self.output;
            let buffer = &mut self.buffer;
            tokio::select! {
                result = output.read_until(b'\0', buffer) => Some(result.map_err(|error| error.to_string())?),
                _ = cancellation.cancelled() => None,
            }
        };
        let read = match read {
            Some(read) => read,
            None => {
                self.stop().await;
                return Err("Search cancelled.".into());
            }
        };
        if read == 0 {
            self.finished = true;
            let status = self.child.wait().await.map_err(|error| error.to_string())?;
            if !status.success() && status.code() != Some(1) {
                return Err("ripgrep failed while searching files.".into());
            }
            return Ok(None);
        }
        if self.buffer.last() == Some(&b'\0') {
            self.buffer.pop();
        }
        Ok(Some(String::from_utf8_lossy(&self.buffer).into_owned()))
    }

    async fn stop(&mut self) {
        if self.finished {
            return;
        }
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        self.finished = true;
    }
}

#[derive(Clone)]
struct ExcludeMatcher {
    set: Option<GlobSet>,
}

impl ExcludeMatcher {
    fn new(parts: &[String]) -> Result<Self, String> {
        if parts.is_empty() {
            return Ok(Self { set: None });
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in parts.iter().flat_map(|part| exclude_globs(part.clone())) {
            builder.add(
                GlobBuilder::new(&pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|error| error.to_string())?,
            );
        }
        Ok(Self {
            set: Some(builder.build().map_err(|error| error.to_string())?),
        })
    }

    fn matches(&self, path: &Path, file_name: &str) -> bool {
        self.set
            .as_ref()
            .is_some_and(|set| set.is_match(path) || set.is_match(file_name))
    }
}

fn exclude_globs(pattern: String) -> Vec<String> {
    vec![
        pattern.clone(),
        format!("**/{pattern}"),
        format!("{pattern}/**"),
        format!("**/{pattern}/**"),
    ]
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::search_cmd::core::query::{EntryTypes, NameMatchMode};
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn signal() -> CancellationSignal {
        CancellationSignal::new()
    }

    fn content_query(root_dir: std::path::PathBuf) -> SearchQuery {
        SearchQuery {
            root_dir,
            file_name: String::new(),
            content_text: "needle".into(),
            content_not_containing: String::new(),
            exclude_pattern: String::new(),
            max_depth: None,
            mode: NameMatchMode::Substring,
            entry_types: EntryTypes::Files,
            content_fixed_string: true,
            name_case_sensitive: false,
            content_case_sensitive: false,
            follow_symlinks: false,
            include_hidden: false,
            search_in_archives: false,
            archive_search_contents: false,
            max_results: 100,
        }
    }

    fn ripgrep_is_available() -> bool {
        std::process::Command::new("rg")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    #[test]
    fn name_search_respects_depth_and_excludes() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("top.rs"), "top").unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/deep.rs"), "deep").unwrap();
        fs::create_dir(temp.path().join("target")).unwrap();
        fs::write(temp.path().join("target/ignored.rs"), "ignored").unwrap();
        let query = SearchQuery {
            root_dir: temp.path().to_path_buf(),
            file_name: "*.rs".into(),
            content_text: String::new(),
            content_not_containing: String::new(),
            exclude_pattern: "target".into(),
            max_depth: Some(0),
            mode: NameMatchMode::Glob,
            entry_types: EntryTypes::Files,
            content_fixed_string: true,
            name_case_sensitive: false,
            content_case_sensitive: false,
            follow_symlinks: false,
            include_hidden: false,
            search_in_archives: false,
            archive_search_contents: false,
            max_results: 100,
        };

        let result = NameSearchExecutor::execute(query, signal()).unwrap();
        assert_eq!(result.paths.len(), 1);
        assert!(result.paths[0].ends_with("top.rs"));
    }

    #[test]
    fn name_search_prunes_files_below_an_excluded_directory() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("target")).unwrap();
        fs::create_dir(temp.path().join("target/deep")).unwrap();
        fs::write(temp.path().join("target/deep/ignored.rs"), "ignored").unwrap();
        fs::write(temp.path().join("kept.rs"), "kept").unwrap();
        let query = SearchQuery {
            root_dir: temp.path().to_path_buf(),
            file_name: "*.rs".into(),
            content_text: String::new(),
            content_not_containing: String::new(),
            exclude_pattern: "target".into(),
            max_depth: None,
            mode: NameMatchMode::Glob,
            entry_types: EntryTypes::Files,
            content_fixed_string: true,
            name_case_sensitive: false,
            content_case_sensitive: false,
            follow_symlinks: false,
            include_hidden: false,
            search_in_archives: false,
            archive_search_contents: false,
            max_results: 100,
        };

        let result = NameSearchExecutor::execute(query, signal()).unwrap();
        assert_eq!(result.paths.len(), 1);
        assert!(result.paths[0].ends_with("kept.rs"));
    }

    #[tokio::test]
    async fn content_search_applies_not_containing_before_the_result_limit() {
        if !ripgrep_is_available() {
            return;
        }
        let temp = tempdir().unwrap();
        for index in 0..5 {
            fs::write(
                temp.path().join(format!("blocked-{index}.txt")),
                "needle forbidden",
            )
            .unwrap();
        }
        fs::write(temp.path().join("allowed-one.txt"), "needle okay").unwrap();
        fs::write(temp.path().join("allowed-two.txt"), "needle okay").unwrap();
        let mut query = content_query(temp.path().to_path_buf());
        query.content_not_containing = "forbidden".into();
        query.max_results = 2;

        let result = ContentSearchExecutor::execute(&query, &signal())
            .await
            .unwrap();
        assert_eq!(result.paths.len(), 2);
        assert!(result.paths.iter().all(|path| path.contains("allowed-")));
    }

    #[tokio::test]
    async fn content_search_honors_regex_filename_mode_and_preserves_spaces() {
        if !ripgrep_is_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let kept = temp.path().join(" leading.rs ");
        fs::write(&kept, "needle").unwrap();
        fs::write(temp.path().join("ignored.txt"), "needle").unwrap();
        let mut query = content_query(temp.path().to_path_buf());
        query.file_name = r".*\.rs\s*".into();
        query.mode = NameMatchMode::Regex;

        let result = ContentSearchExecutor::execute(&query, &signal())
            .await
            .unwrap();
        assert_eq!(result.paths, vec![kept.to_string_lossy().into_owned()]);
    }

    #[test]
    fn archive_search_uses_the_same_bounded_result_model() {
        let temp = tempdir().unwrap();
        let archive_path = temp.path().join("fixture.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("docs/kept.txt", options).unwrap();
        writer.write_all(b"needle").unwrap();
        writer.start_file("docs/blocked.log", options).unwrap();
        writer.write_all(b"needle").unwrap();
        writer.finish().unwrap();

        let mut query = content_query(temp.path().to_path_buf());
        query.search_in_archives = true;
        query.archive_search_contents = true;
        query.exclude_pattern = "*.log".into();
        let hits = ArchiveSearchExecutor::execute(query, signal(), 200).unwrap();

        assert_eq!(
            hits,
            vec![format!("{}#docs/kept.txt", archive_path.display())]
        );
    }
}
