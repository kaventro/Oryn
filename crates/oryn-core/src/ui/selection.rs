use std::collections::HashSet;

use regex::Regex;

pub trait Entry {
    fn base(&self) -> &str;
    fn is_dir(&self) -> bool;
    fn size(&self) -> u64;
}

fn selectable<E: Entry>(e: &E) -> bool {
    !e.base().is_empty() && e.base() != ".."
}

pub fn mask_to_regex(mask: &str) -> Regex {
    let mut pattern = String::from("(?i)^");
    for c in mask.chars() {
        match c {
            '*' => pattern.push_str(".*"),
            '?' => pattern.push('.'),
            c => pattern.push_str(&regex::escape(&c.to_string())),
        }
    }
    pattern.push('$');
    Regex::new(&pattern).expect("escaped mask is a valid regex")
}

pub fn file_extension<E: Entry>(e: &E) -> Option<String> {
    if e.base() == ".." || e.is_dir() {
        return None;
    }
    let base = e.base();
    match base.rfind('.') {
        Some(i) if i > 0 && i < base.len() - 1 => Some(base[i + 1..].to_lowercase()),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionSummary {
    pub sel_count: usize,
    pub sel_bytes: u64,
    pub sel_dirs: usize,
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Selection {
    bases: HashSet<String>,
}

impl Selection {
    pub fn contains(&self, base: &str) -> bool {
        self.bases.contains(base)
    }

    pub fn len(&self) -> usize {
        self.bases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.bases.iter().map(String::as_str)
    }

    pub fn retain<E: Entry>(&mut self, items: &[E]) {
        let present: HashSet<&str> = items.iter().map(|e| e.base()).collect();
        self.bases.retain(|b| present.contains(b.as_str()));
    }

    pub fn toggle(&mut self, base: &str) -> bool {
        if !self.bases.remove(base) {
            self.bases.insert(base.to_string());
            return true;
        }
        false
    }

    pub fn apply_mask<E: Entry>(&mut self, items: &[E], mask: &str, selecting: bool) -> String {
        let re = mask_to_regex(mask);
        let mut hit = 0;
        for item in items.iter().filter(|e| selectable(*e)) {
            if !re.is_match(item.base()) {
                continue;
            }
            hit += 1;
            if selecting {
                self.bases.insert(item.base().to_string());
            } else {
                self.bases.remove(item.base());
            }
        }
        let verb = if selecting { "Selected" } else { "Deselected" };
        format!(
            "{verb} by {mask}: {hit} matched, {} total.",
            self.bases.len()
        )
    }

    pub fn invert<E: Entry>(&mut self, items: &[E]) -> String {
        for item in items.iter().filter(|e| selectable(*e)) {
            self.toggle(item.base());
        }
        format!("Selection inverted: {} selected.", self.bases.len())
    }

    pub fn select_all<E: Entry>(&mut self, items: &[E]) -> String {
        for item in items.iter().filter(|e| selectable(*e)) {
            self.bases.insert(item.base().to_string());
        }
        format!("Selected all: {}.", self.bases.len())
    }

    pub fn clear(&mut self) -> String {
        self.bases.clear();
        "Selection cleared.".to_string()
    }

    pub fn select_by_extension<E: Entry>(
        &mut self,
        items: &[E],
        cursor: Option<&E>,
    ) -> Option<String> {
        let cursor = cursor.filter(|e| selectable(*e))?;
        let Some(ext) = file_extension(cursor) else {
            return Some("Cursor item has no extension.".to_string());
        };
        let mut hit = 0;
        for item in items.iter().filter(|e| selectable(*e) && !e.is_dir()) {
            if file_extension(item).as_deref() == Some(ext.as_str()) {
                self.bases.insert(item.base().to_string());
                hit += 1;
            }
        }
        Some(format!(
            "Selected *.{ext}: {hit} file(s), {} total.",
            self.bases.len()
        ))
    }

    pub fn summary<E: Entry>(&self, items: &[E]) -> SelectionSummary {
        let mut out = SelectionSummary::default();
        for item in items.iter().filter(|e| selectable(*e)) {
            if item.is_dir() {
                out.total_dirs += 1;
            } else {
                out.total_files += 1;
            }
            out.total_bytes += item.size();
            if self.bases.contains(item.base()) {
                out.sel_count += 1;
                out.sel_bytes += item.size();
                if item.is_dir() {
                    out.sel_dirs += 1;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item(&'static str, bool, u64);

    impl Entry for Item {
        fn base(&self) -> &str {
            self.0
        }
        fn is_dir(&self) -> bool {
            self.1
        }
        fn size(&self) -> u64 {
            self.2
        }
    }

    #[test]
    fn retain_drops_missing_names() {
        let mut sel = Selection::default();
        sel.toggle("a");
        sel.toggle("gone");
        sel.retain(&[Item("a", false, 1), Item("b", false, 2)]);
        assert_eq!(sel.names().collect::<Vec<_>>(), vec!["a"]);
    }

    #[test]
    fn mask_to_regex_converts_wildcards() {
        let r1 = mask_to_regex("*.ts");
        assert!(r1.is_match("index.ts"));
        assert!(r1.is_match("INDEX.TS"));
        assert!(!r1.is_match("index.js"));
        let r2 = mask_to_regex("test-?.txt");
        assert!(r2.is_match("test-1.txt"));
        assert!(!r2.is_match("test-12.txt"));
        let r3 = mask_to_regex("name[1].txt");
        assert!(r3.is_match("name[1].txt"));
        assert!(!r3.is_match("name1.txt"));
    }

    #[test]
    fn mask_invert_select_all_and_clear() {
        let items = [
            Item("..", true, 0),
            Item("file1.txt", false, 100),
            Item("file2.png", false, 200),
            Item("file3.txt", false, 300),
            Item("subfolder", true, 0),
        ];
        let mut sel = Selection::default();
        let msg = sel.apply_mask(&items, "*.txt", true);
        assert!(sel.contains("file1.txt") && sel.contains("file3.txt"));
        assert!(!sel.contains("file2.png") && !sel.contains(".."));
        assert_eq!(msg, "Selected by *.txt: 2 matched, 2 total.");

        let msg = sel.apply_mask(&items, "file1.*", false);
        assert!(!sel.contains("file1.txt") && sel.contains("file3.txt"));
        assert!(msg.starts_with("Deselected by file1.*"));

        sel.invert(&items);
        assert!(!sel.contains("file3.txt"));
        assert!(
            sel.contains("file1.txt") && sel.contains("file2.png") && sel.contains("subfolder")
        );
        assert!(!sel.contains(".."));

        sel.select_all(&items);
        assert_eq!(sel.len(), 4);

        assert_eq!(sel.clear(), "Selection cleared.");
        assert!(sel.is_empty());
    }

    #[test]
    fn select_by_extension_skips_folders() {
        let items = [
            Item("..", true, 0),
            Item("app.min.js", false, 500),
            Item("bundle.min.js", false, 600),
            Item("style.css", false, 100),
            Item("noext", false, 50),
            Item("folder.js", true, 0),
        ];
        let mut sel = Selection::default();
        assert_eq!(sel.select_by_extension(&items, None), None);
        assert_eq!(sel.select_by_extension(&items, Some(&items[0])), None);
        assert_eq!(
            sel.select_by_extension(&items, Some(&items[4])).as_deref(),
            Some("Cursor item has no extension.")
        );
        let msg = sel.select_by_extension(&items, Some(&items[1])).unwrap();
        assert!(sel.contains("app.min.js") && sel.contains("bundle.min.js"));
        assert!(!sel.contains("folder.js"));
        assert!(msg.starts_with("Selected *.js"));
    }

    #[test]
    fn summary_counts_files_dirs_and_bytes() {
        let items = [
            Item("..", true, 0),
            Item("doc1.txt", false, 1000),
            Item("doc2.txt", false, 2000),
            Item("photos", true, 0),
        ];
        let mut sel = Selection::default();
        sel.toggle("doc1.txt");
        sel.toggle("photos");
        let sum = sel.summary(&items);
        assert_eq!(
            sum,
            SelectionSummary {
                sel_count: 2,
                sel_bytes: 1000,
                sel_dirs: 1,
                total_files: 2,
                total_dirs: 1,
                total_bytes: 3000,
            }
        );
    }

    #[test]
    fn file_extension_matches_the_web_helper() {
        assert_eq!(file_extension(&Item("..", false, 0)), None);
        assert_eq!(file_extension(&Item("folder", true, 0)), None);
        assert_eq!(file_extension(&Item("noext", false, 0)), None);
        assert_eq!(file_extension(&Item(".dot", false, 0)), None);
        assert_eq!(
            file_extension(&Item("doc.PDF", false, 0)).as_deref(),
            Some("pdf")
        );
    }
}
