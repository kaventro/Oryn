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

fn name_cmp(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

pub fn arrange<E: ArrangeEntry>(items: &[E], filter: &str, order: SortOrder) -> Vec<usize> {
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

    let parent = items.iter().position(|it| it.base() == "..");
    let mut rest: Vec<usize> = (0..items.len()).filter(|&i| Some(i) != parent).collect();
    rest.sort_by(|&ia, &ib| {
        let (a, b) = (&items[ia], &items[ib]);
        if a.is_dir() != b.is_dir() {
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
            names(&items, arrange(&items, "app", order)),
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
        let sort =
            |field, ascending| names(&items, arrange(&items, "", SortOrder { field, ascending }));
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
