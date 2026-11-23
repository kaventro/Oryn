use regex::Regex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaseTransform {
    #[default]
    None,
    Lower,
    Upper,
    Title,
    First,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumberPosition {
    #[default]
    None,
    Prefix,
    Suffix,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiRename {
    pub find: String,
    pub replace: String,
    pub regex: bool,
    pub prefix: String,
    pub suffix: String,
    pub case: CaseTransform,
    pub number: NumberPosition,
    pub start: u64,
    pub digits: usize,
}

impl Default for MultiRename {
    fn default() -> Self {
        Self {
            find: String::new(),
            replace: String::new(),
            regex: false,
            prefix: String::new(),
            suffix: String::new(),
            case: CaseTransform::None,
            number: NumberPosition::None,
            start: 1,
            digits: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Day {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Day {
    pub fn today() -> Self {
        use chrono::Datelike;
        let now = chrono::Local::now();
        Self {
            year: now.year(),
            month: now.month(),
            day: now.day(),
        }
    }
}

pub fn split_name(base: &str) -> (&str, &str) {
    match base.rfind('.') {
        Some(i) if i > 0 && i < base.len() - 1 => (&base[..i], &base[i..]),
        _ => (base, ""),
    }
}

impl MultiRename {
    fn counter(&self, index: usize) -> String {
        format!("{:0width$}", self.start + index as u64, width = self.digits)
    }

    fn tokens(&self, text: &str, stem: &str, ext: &str, index: usize, day: Day) -> String {
        if text.is_empty() {
            return String::new();
        }
        let ymd = format!("{:04}-{:02}-{:02}", day.year, day.month, day.day);
        text.replace("[N]", stem)
            .replace("[E]", ext.trim_start_matches('.'))
            .replace("[C]", &self.counter(index))
            .replace("[YMD]", &ymd)
            .replace("[Y]", &day.year.to_string())
            .replace("[M]", &format!("{:02}", day.month))
            .replace("[D]", &format!("{:02}", day.day))
    }

    pub fn apply(&self, original: &str, index: usize, day: Day) -> String {
        let (stem, ext) = split_name(original);
        let mut ext = ext.to_string();
        let mut result = stem.to_string();

        if !self.find.is_empty() {
            let rep = self.tokens(&self.replace, stem, &ext, index, day);
            if self.regex {
                if let Ok(re) = Regex::new(&self.find) {
                    result = re.replace_all(&result, rep.as_str()).into_owned();
                }
            } else {
                result = result.replace(&self.find, &rep);
            }
        }

        match self.case {
            CaseTransform::None => {}
            CaseTransform::Lower => {
                result = result.to_lowercase();
                ext = ext.to_lowercase();
            }
            CaseTransform::Upper => {
                result = result.to_uppercase();
                ext = ext.to_uppercase();
            }
            CaseTransform::Title => result = title_case(&result),
            CaseTransform::First => {
                let lower = result.to_lowercase();
                let mut chars = lower.chars();
                result = match chars.next() {
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                    None => String::new(),
                };
            }
        }

        match self.number {
            NumberPosition::None => {}
            NumberPosition::Prefix => result = format!("{}_{result}", self.counter(index)),
            NumberPosition::Suffix => result = format!("{result}_{}", self.counter(index)),
        }

        if !self.prefix.is_empty() {
            result = self.tokens(&self.prefix, stem, &ext, index, day) + &result;
        }
        if !self.suffix.is_empty() {
            result += &self.tokens(&self.suffix, stem, &ext, index, day);
        }
        result + &ext
    }
}

fn title_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_word = false;
    for c in text.chars() {
        let word = c.is_alphanumeric() || c == '_';
        if word && !prev_word {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        prev_word = word;
    }
    out
}

pub fn duplicate_name(base: &str, is_dir: bool, exists: impl Fn(&str) -> bool) -> String {
    let (stem, ext) = if is_dir { (base, "") } else { split_name(base) };
    let mut candidate = format!("{stem} - Copy{ext}");
    let mut counter = 2;
    while exists(&candidate) {
        candidate = format!("{stem} - Copy {counter}{ext}");
        counter += 1;
    }
    candidate
}

pub fn validate_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name.".into());
    }
    if name == "." || name == ".." {
        return Err(format!("\u{201c}{name}\u{201d} can't be used as a name."));
    }
    if name.contains('/')
        || (cfg!(windows) && name.contains(['\\', ':', '*', '?', '"', '<', '>', '|']))
    {
        return Err("Names can't contain a slash.".into());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: Day = Day {
        year: 2026,
        month: 3,
        day: 7,
    };

    #[test]
    fn split_keeps_dotfiles_whole() {
        assert_eq!(split_name("a.txt"), ("a", ".txt"));
        assert_eq!(split_name(".bashrc"), (".bashrc", ""));
        assert_eq!(split_name("trailing."), ("trailing.", ""));
        assert_eq!(split_name("a.tar.gz"), ("a.tar", ".gz"));
    }

    #[test]
    fn default_options_keep_name() {
        assert_eq!(
            MultiRename::default().apply("photo.JPG", 0, DAY),
            "photo.JPG"
        );
    }

    #[test]
    fn find_replace_plain_and_regex() {
        let plain = MultiRename {
            find: "IMG".into(),
            replace: "trip-[C]".into(),
            ..Default::default()
        };
        assert_eq!(plain.apply("IMG_IMG.png", 2, DAY), "trip-03_trip-03.png");
        let re = MultiRename {
            find: r"\d+".into(),
            replace: "#".into(),
            regex: true,
            ..Default::default()
        };
        assert_eq!(re.apply("a1b22.txt", 0, DAY), "a#b#.txt");
        let bad = MultiRename {
            find: "(".into(),
            regex: true,
            ..Default::default()
        };
        assert_eq!(bad.apply("x(.txt", 0, DAY), "x(.txt");
    }

    #[test]
    fn case_transforms() {
        let mk = |case| MultiRename {
            case,
            ..Default::default()
        };
        assert_eq!(
            mk(CaseTransform::Lower).apply("Hello World.TXT", 0, DAY),
            "hello world.txt"
        );
        assert_eq!(
            mk(CaseTransform::Upper).apply("Hello world.txt", 0, DAY),
            "HELLO WORLD.TXT"
        );
        assert_eq!(
            mk(CaseTransform::Title).apply("hello big-world.txt", 0, DAY),
            "Hello Big-World.txt"
        );
        assert_eq!(
            mk(CaseTransform::First).apply("hELLO World.txt", 0, DAY),
            "Hello world.txt"
        );
        assert_eq!(mk(CaseTransform::First).apply(".txt", 0, DAY), ".txt");
    }

    #[test]
    fn numbering_and_tokens() {
        let opts = MultiRename {
            number: NumberPosition::Prefix,
            start: 9,
            digits: 3,
            prefix: "[YMD] ".into(),
            suffix: " [Y]-[M]-[D] [E]".into(),
            ..Default::default()
        };
        assert_eq!(
            opts.apply("a.md", 1, DAY),
            "2026-03-07 010_a 2026-03-07 md.md"
        );
        let suffix = MultiRename {
            number: NumberPosition::Suffix,
            ..Default::default()
        };
        assert_eq!(suffix.apply("n", 0, DAY), "n_01");
        let name = MultiRename {
            prefix: "[N]-".into(),
            ..Default::default()
        };
        assert_eq!(name.apply("x.y", 0, DAY), "x-x.y");
    }

    #[test]
    fn today_is_plausible() {
        let d = Day::today();
        assert!(d.year >= 2024 && (1..=12).contains(&d.month) && (1..=31).contains(&d.day));
    }

    #[test]
    fn duplicate_names_count_up() {
        let taken = ["a - Copy.txt", "a - Copy 2.txt"];
        assert_eq!(
            duplicate_name("a.txt", false, |n| taken.contains(&n)),
            "a - Copy 3.txt"
        );
        assert_eq!(duplicate_name("dir.d", true, |_| false), "dir.d - Copy");
        assert_eq!(duplicate_name(".env", false, |_| false), ".env - Copy");
    }

    #[test]
    fn names_are_validated() {
        assert_eq!(validate_name("  ok.txt "), Ok("ok.txt"));
        assert!(validate_name("").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("a/b").is_err());
    }
}
