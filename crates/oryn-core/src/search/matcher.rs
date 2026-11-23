pub type NameMatcher = Box<dyn Fn(&str, bool) -> bool + Send + Sync>;

pub fn build_name_matcher(
    parts: &[String],
    mode: &str,
    case_sensitive: bool,
) -> Result<NameMatcher, String> {
    if parts.is_empty() {
        return Ok(Box::new(|_n, _d| false));
    }
    match mode {
        "exact" => {
            let p = parts.to_vec();
            Ok(Box::new(move |n: &str, _d: bool| {
                p.iter().any(|q| {
                    if case_sensitive {
                        n == q.as_str()
                    } else {
                        n.eq_ignore_ascii_case(q)
                    }
                })
            }))
        }
        "substring" => {
            let p = parts.to_vec();
            Ok(Box::new(move |n: &str, _d: bool| {
                p.iter().any(|q| {
                    if case_sensitive {
                        n.contains(q.as_str())
                    } else {
                        n.to_lowercase().contains(&q.to_lowercase())
                    }
                })
            }))
        }
        "glob" => {
            let mut builder = globset::GlobSetBuilder::new();
            for pattern in parts {
                let glob = globset::GlobBuilder::new(pattern)
                    .case_insensitive(!case_sensitive)
                    .build()
                    .map_err(|e| e.to_string())?;
                builder.add(glob);
            }
            let set = builder.build().map_err(|e| e.to_string())?;
            Ok(Box::new(move |n: &str, _d: bool| set.is_match(n)))
        }
        "regex" => {
            let joined = parts.join("|");
            let re = regex::RegexBuilder::new(&joined)
                .case_insensitive(!case_sensitive)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Box::new(move |n: &str, _d: bool| re.is_match(n)))
        }
        _ => {
            let p = parts.to_vec();
            Ok(Box::new(move |n: &str, _d: bool| {
                p.iter()
                    .any(|q| n.to_lowercase().contains(&q.to_lowercase()))
            }))
        }
    }
}
