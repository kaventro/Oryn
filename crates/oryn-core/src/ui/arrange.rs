use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortField {
    #[default]
    Name,
    Ext,
    Size,
    Date,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortOrder {
    pub field: SortField,
    pub ascending: bool,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self {
            field: SortField::Name,
            ascending: true,
        }
    }
}

impl SortOrder {
    pub fn toggled(self, field: SortField) -> Self {
        if self.field == field {
            Self {
                field,
                ascending: !self.ascending,
            }
        } else {
            Self {
                field,
                ascending: true,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FoldersOnTop {
    #[default]
    Always,
    Never,
    ExceptDate,
}

impl FoldersOnTop {
    pub fn applies(self, field: SortField) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::ExceptDate => field != SortField::Date,
        }
    }
}

pub trait ArrangeEntry {
    fn base(&self) -> &str;
    fn is_dir(&self) -> bool;
    fn size(&self) -> u64;
    fn mtime_ms(&self) -> i64;
}

pub fn fuzzy_score(base: &str, query: &str) -> i64 {
    let b = base.to_lowercase();
    let q = query.to_lowercase();
    if q.is_empty() {
        return 1;
    }
    if b.starts_with(&q) {
        return 1000 - b.chars().count() as i64;
    }
    if let Some(idx) = b.find(&q) {
        return 500 - b[..idx].chars().count() as i64;
    }
    let mut rest = b.as_str();
    for c in q.chars() {
        match rest.find(c) {
            Some(i) => rest = &rest[i + c.len_utf8()..],
            None => return 0,
        }
    }
    100
}

pub fn short_extension(base: &str, is_dir: bool) -> String {
    if base == ".." || is_dir {
        return "\u{2014}".into();
    }
    match base.rfind('.') {
        Some(i) if i > 0 && i < base.len() - 1 => {
            base[i + 1..].to_lowercase().chars().take(12).collect()
        }
        _ => "\u{2014}".into(),
    }
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (la, lb) = (a.to_lowercase(), b.to_lowercase());
    let (mut x, mut y) = (la.as_str(), lb.as_str());
    loop {
        match (x.chars().next(), y.chars().next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let ex = x.find(|ch: char| !ch.is_ascii_digit()).unwrap_or(x.len());
                let ey = y.find(|ch: char| !ch.is_ascii_digit()).unwrap_or(y.len());
                let (nx, ny) = (
                    x[..ex].trim_start_matches('0'),
                    y[..ey].trim_start_matches('0'),
                );
                let ord = nx.len().cmp(&ny.len()).then_with(|| nx.cmp(ny));
                if ord != Ordering::Equal {
                    return ord;
                }
                x = &x[ex..];
                y = &y[ey..];
            }
            (Some(c), Some(d)) => {
                if c != d {
                    return c.cmp(&d);
                }
                x = &x[c.len_utf8()..];
                y = &y[d.len_utf8()..];
            }
        }
    }
}

fn name_cmp(a: &str, b: &str) -> Ordering {
    natural_cmp(a, b).then_with(|| a.cmp(b))
}

pub fn arrange<E: ArrangeEntry>(
    items: &[E],
    filter: &str,
    order: SortOrder,
    folders: FoldersOnTop,
) -> Vec<usize> {
    let filter = filter.trim();
    if !filter.is_empty() {
        let mut scored: Vec<(usize, i64)> = items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| {
                let score = if it.base() == ".." {
                    9999
                } else {
                    fuzzy_score(it.base(), filter)
                };
                (score > 0).then_some((i, score))
            })
            .collect();
        scored.sort_by_key(|s| std::cmp::Reverse(s.1));
        return scored.into_iter().map(|(i, _)| i).collect();
    }

    let folders_first = folders.applies(order.field);
    let parent = items.iter().position(|it| it.base() == "..");
    let mut rest: Vec<usize> = (0..items.len()).filter(|&i| Some(i) != parent).collect();
    rest.sort_by(|&ia, &ib| {
        let (a, b) = (&items[ia], &items[ib]);
        if folders_first && a.is_dir() != b.is_dir() {
            return if a.is_dir() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let primary = match order.field {
            SortField::Name => Ordering::Equal,
            SortField::Ext => {
                short_extension(a.base(), a.is_dir()).cmp(&short_extension(b.base(), b.is_dir()))
            }
            SortField::Size => a.size().cmp(&b.size()),
            SortField::Date => a.mtime_ms().cmp(&b.mtime_ms()),
        };
        let ord = primary.then_with(|| name_cmp(a.base(), b.base()));
        if order.ascending {
            ord
        } else {
            ord.reverse()
        }
    });
    parent.into_iter().chain(rest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_puts_numbers_in_sequence() {
        let mut v = vec!["file10.txt", "File2.txt", "file1.txt", "file02b", "a"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["a", "file1.txt", "File2.txt", "file02b", "file10.txt"]);
    }

    struct It(&'static str, bool, u64, i64);

    impl ArrangeEntry for It {
        fn base(&self) -> &str {
            self.0
        }
        fn is_dir(&self) -> bool {
            self.1
        }
        fn size(&self) -> u64 {
            self.2
        }
        fn mtime_ms(&self) -> i64 {
            self.3
        }
    }

    fn names(items: &[It], idx: Vec<usize>) -> Vec<&'static str> {
        idx.into_iter().map(|i| items[i].0).collect()
    }

    #[test]
    fn fuzzy_score_matches_the_web_scoring() {
        assert_eq!(fuzzy_score("test.ts", ""), 1);
        assert_eq!(fuzzy_score("apple", "app"), 995);
        assert_eq!(fuzzy_score("pineapple", "app"), 496);
        assert_eq!(fuzzy_score("axpxpxlxe", "apple"), 100);
        assert_eq!(fuzzy_score("apple", "banana"), 0);
    }

    #[test]
    fn filter_keeps_parent_first_and_ranks_by_score() {
        let items = [
            It("..", true, 0, 0),
            It("app.ts", false, 0, 0),
            It("apple.png", false, 0, 0),
            It("zebra.txt", false, 0, 0),
        ];
        let order = SortOrder::default();
        assert_eq!(
            names(&items, arrange(&items, "app", order, FoldersOnTop::Always)),
            ["..", "app.ts", "apple.png"]
        );
    }

    #[test]
    fn sorts_by_each_field_with_folders_first() {
        let items = [
            It("z_folder", true, 50, 1000),
            It("a_folder", true, 10, 5000),
            It("file_b.png", false, 300, 1_767_225_600_000),
            It("file_a.txt", false, 100, 1_769_904_000_000),
            It("file_c.doc", false, 200, 0),
            It("..", true, 0, 0),
        ];
        let sort = |field, ascending| {
            names(
                &items,
                arrange(
                    &items,
                    "",
                    SortOrder { field, ascending },
                    FoldersOnTop::Always,
                ),
            )
        };
        assert_eq!(
            sort(SortField::Name, true),
            [
                "..",
                "a_folder",
                "z_folder",
                "file_a.txt",
                "file_b.png",
                "file_c.doc"
            ]
        );
        assert_eq!(
            sort(SortField::Name, false),
            [
                "..",
                "z_folder",
                "a_folder",
                "file_c.doc",
                "file_b.png",
                "file_a.txt"
            ]
        );
        assert_eq!(
            sort(SortField::Ext, true)[3..],
            ["file_c.doc", "file_b.png", "file_a.txt"]
        );
        assert_eq!(
            sort(SortField::Size, true)[3..],
            ["file_a.txt", "file_c.doc", "file_b.png"]
        );
        assert_eq!(
            sort(SortField::Date, true)[3..],
            ["file_c.doc", "file_b.png", "file_a.txt"]
        );
    }

    #[test]
    fn folders_mix_with_files_when_not_kept_on_top() {
        let items = [
            It("..", true, 0, 0),
            It("old_folder", true, 0, 1000),
            It("new_file.txt", false, 400, 9000),
            It("new_folder", true, 0, 8000),
            It("old_file.txt", false, 100, 2000),
        ];
        let sort = |field, ascending, folders| {
            names(
                &items,
                arrange(&items, "", SortOrder { field, ascending }, folders),
            )
        };
        assert_eq!(
            sort(SortField::Date, false, FoldersOnTop::Always),
            [
                "..",
                "new_folder",
                "old_folder",
                "new_file.txt",
                "old_file.txt"
            ]
        );
        assert_eq!(
            sort(SortField::Date, false, FoldersOnTop::Never),
            [
                "..",
                "new_file.txt",
                "new_folder",
                "old_file.txt",
                "old_folder"
            ]
        );
        assert_eq!(
            sort(SortField::Date, false, FoldersOnTop::ExceptDate),
            [
                "..",
                "new_file.txt",
                "new_folder",
                "old_file.txt",
                "old_folder"
            ]
        );
        assert_eq!(
            sort(SortField::Name, true, FoldersOnTop::ExceptDate),
            [
                "..",
                "new_folder",
                "old_folder",
                "new_file.txt",
                "old_file.txt"
            ]
        );
        assert_eq!(
            sort(SortField::Name, true, FoldersOnTop::Never),
            [
                "..",
                "new_file.txt",
                "new_folder",
                "old_file.txt",
                "old_folder"
            ]
        );
        assert_eq!(
            sort(SortField::Size, true, FoldersOnTop::Never)[1..3],
            ["new_folder", "old_folder"]
        );
    }

    #[test]
    fn clicking_a_column_twice_reverses_it() {
        let o = SortOrder::default().toggled(SortField::Size);
        assert_eq!(
            o,
            SortOrder {
                field: SortField::Size,
                ascending: true
            }
        );
        assert!(!o.toggled(SortField::Size).ascending);
        assert_eq!(short_extension("Archive.TAR", false), "tar");
        assert_eq!(short_extension("folder", true), "\u{2014}");
    }
}
