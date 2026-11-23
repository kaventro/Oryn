use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Image,
    Markdown,
    Code(Lang),
    Text,
    Office,
    Audio,
    Video,
    Pdf,
    Binary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    CLike,
    Python,
    Shell,
    Data,
    Markup,
    Sql,
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn kind_of(path: &Path) -> Kind {
    let ext = extension(path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "svg" | "tif" | "tiff" => {
            Kind::Image
        }
        "md" | "markdown" | "mdx" => Kind::Markdown,
        "rs" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "java" | "kt" | "swift" | "go" | "js"
        | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "dart" | "scala" | "php" | "zig" | "css"
        | "scss" | "less" => Kind::Code(Lang::CLike),
        "py" | "rb" | "pl" | "r" | "lua" | "ex" | "exs" => Kind::Code(Lang::Python),
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => Kind::Code(Lang::Shell),
        "json" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "env" | "lock" => {
            Kind::Code(Lang::Data)
        }
        "html" | "htm" | "xml" | "svelte" | "vue" | "plist" => Kind::Code(Lang::Markup),
        "sql" => Kind::Code(Lang::Sql),
        "txt" | "log" | "csv" | "tsv" | "rst" | "adoc" | "srt" => Kind::Text,
        "docx" | "xlsx" | "pptx" | "odt" | "ods" | "odp" => Kind::Office,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" | "aiff" => Kind::Audio,
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Kind::Video,
        "pdf" => Kind::Pdf,
        _ if matches!(
            name.as_str(),
            "makefile" | "dockerfile" | "license" | "readme" | ".gitignore" | ".editorconfig"
        ) =>
        {
            Kind::Text
        }
        _ => {
            if looks_text(path) {
                Kind::Text
            } else {
                Kind::Binary
            }
        }
    }
}

fn looks_text(path: &Path) -> bool {
    let mut buf = [0u8; 4096];
    let Ok(n) = File::open(path).and_then(|mut f| f.read(&mut buf)) else {
        return false;
    };
    let head = &buf[..n];
    !head.contains(&0)
        && match std::str::from_utf8(head) {
            Ok(_) => true,
            Err(e) => e.error_len().is_none() && head.len() - e.valid_up_to() < 4,
        }
}

pub fn read_text(path: &Path, max: usize) -> std::io::Result<(String, bool)> {
    let mut file = File::open(path)?;
    let mut buf = Vec::with_capacity(max.min(1 << 20));
    file.by_ref().take(max as u64 + 1).read_to_end(&mut buf)?;
    let truncated = buf.len() > max;
    buf.truncate(max);
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        while text.ends_with('\u{fffd}') {
            text.pop();
        }
    }
    Ok((text.replace("\r\n", "\n"), truncated))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexLine {
    pub offset: u64,
    pub hex: String,
    pub ascii: String,
}

pub fn hex_dump(path: &Path, offset: u64, len: usize) -> std::io::Result<Vec<HexLine>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    let mut read = 0;
    while read < len {
        let n = file.read(&mut buf[read..])?;
        if n == 0 {
            break;
        }
        read += n;
    }
    buf.truncate(read);
    Ok(buf
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            let mut hex = String::with_capacity(48);
            for (j, b) in chunk.iter().enumerate() {
                if j == 8 {
                    hex.push(' ');
                }
                hex.push_str(&format!("{b:02x} "));
            }
            let ascii = chunk
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            HexLine {
                offset: offset + (i * 16) as u64,
                hex: hex.trim_end().to_string(),
                ascii,
            }
        })
        .collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Punct,
    Type,
}

const CLIKE: &[&str] = &[
    "fn",
    "let",
    "mut",
    "pub",
    "use",
    "mod",
    "struct",
    "enum",
    "impl",
    "trait",
    "match",
    "if",
    "else",
    "for",
    "while",
    "loop",
    "return",
    "break",
    "continue",
    "const",
    "static",
    "async",
    "await",
    "move",
    "where",
    "self",
    "Self",
    "super",
    "crate",
    "as",
    "in",
    "dyn",
    "ref",
    "type",
    "true",
    "false",
    "null",
    "function",
    "var",
    "class",
    "extends",
    "import",
    "export",
    "from",
    "new",
    "this",
    "void",
    "int",
    "char",
    "float",
    "double",
    "bool",
    "string",
    "package",
    "func",
    "interface",
    "private",
    "public",
    "protected",
    "default",
    "case",
    "switch",
    "try",
    "catch",
    "throw",
    "finally",
    "typeof",
    "instanceof",
    "undefined",
    "unsafe",
    "extern",
    "defer",
    "go",
    "chan",
    "nil",
    "goto",
    "do",
    "of",
];

const PYTHON: &[&str] = &[
    "def", "class", "import", "from", "as", "if", "elif", "else", "for", "while", "return",
    "yield", "with", "try", "except", "finally", "raise", "pass", "break", "continue", "lambda",
    "in", "is", "not", "and", "or", "None", "True", "False", "self", "async", "await", "global",
    "end", "do", "then", "local", "function", "require", "module", "nil",
];

const SHELL: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac",
    "function", "return", "export", "local", "echo", "set", "unset", "exit", "cd", "source",
];

const SQL: &[&str] = &[
    "select", "from", "where", "insert", "into", "values", "update", "set", "delete", "create",
    "table", "drop", "alter", "and", "or", "not", "null", "join", "left", "right", "inner",
    "outer", "on", "group", "by", "order", "having", "limit", "as", "primary", "key", "index",
];

const DATA: &[&str] = &["true", "false", "null", "yes", "no", "on", "off"];

fn keywords(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::CLike => CLIKE,
        Lang::Python => PYTHON,
        Lang::Shell => SHELL,
        Lang::Sql => SQL,
        Lang::Data => DATA,
        Lang::Markup => &[],
    }
}

fn line_comment(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::CLike => &["//"],
        Lang::Python | Lang::Shell | Lang::Data => &["#"],
        Lang::Sql => &["--"],
        Lang::Markup => &[],
    }
}

pub fn highlight(line: &str, lang: Lang, in_block: &mut bool) -> Vec<(Tok, String)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(Tok, String)> = Vec::new();
    let mut push = |t: Tok, s: String| match out.last_mut() {
        Some((lt, ls)) if *lt == t => ls.push_str(&s),
        _ => out.push((t, s)),
    };
    let block = match lang {
        Lang::CLike => Some(("/*", "*/")),
        Lang::Markup => Some(("<!--", "-->")),
        _ => None,
    };
    let starts = |i: usize, pat: &str| -> bool {
        let p: Vec<char> = pat.chars().collect();
        chars.len() >= i + p.len() && chars[i..i + p.len()] == p[..]
    };
    let kws = keywords(lang);
    let sql = lang == Lang::Sql;
    let mut first_word = true;
    let mut i = 0;
    while i < chars.len() {
        if *in_block {
            let (_, close) = block.unwrap_or(("", ""));
            let mut j = i;
            while j < chars.len() && !starts(j, close) {
                j += 1;
            }
            if j < chars.len() {
                j += close.chars().count();
                *in_block = false;
            }
            push(Tok::Comment, chars[i..j].iter().collect());
            i = j;
            continue;
        }
        if let Some((open, _)) = block {
            if starts(i, open) {
                *in_block = true;
                let n = open.chars().count();
                push(Tok::Comment, chars[i..i + n].iter().collect());
                i += n;
                continue;
            }
        }
        if line_comment(lang).iter().any(|c| starts(i, c)) {
            push(Tok::Comment, chars[i..].iter().collect());
            break;
        }
        let c = chars[i];
        if c == '"' || c == '\'' || c == '`' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != c {
                if chars[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            let end = (j + 1).min(chars.len());
            push(Tok::String, chars[i..end].iter().collect());
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len()
                && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_')
            {
                j += 1;
            }
            push(Tok::Number, chars[i..j].iter().collect());
            i = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' || (lang == Lang::Markup && c == '-') {
            let mut j = i;
            while j < chars.len()
                && (chars[j].is_alphanumeric()
                    || chars[j] == '_'
                    || (lang == Lang::Markup && chars[j] == '-'))
            {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            let probe = if sql {
                word.to_ascii_lowercase()
            } else {
                word.clone()
            };
            let tag = lang == Lang::Markup && i > 0 && (chars[i - 1] == '<' || chars[i - 1] == '/');
            let capital = word.chars().next().is_some_and(char::is_uppercase) && lang != Lang::Data;
            let tok = if kws.contains(&probe.as_str()) || tag {
                Tok::Keyword
            } else if capital
                || lang == Lang::Data
                    && chars.get(j..).is_some_and(|r| {
                        r.iter()
                            .find(|c| !c.is_whitespace())
                            .is_some_and(|c| *c == ':' || *c == '=')
                    })
                    && first_word
            {
                Tok::Type
            } else {
                Tok::Plain
            };
            push(tok, word);
            first_word = false;
            i = j;
            continue;
        }
        let tok = if c.is_ascii_punctuation() {
            Tok::Punct
        } else {
            Tok::Plain
        };
        push(tok, c.to_string());
        i += 1;
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, String),
    Paragraph(String),
    Bullet(usize, String),
    Numbered(usize, String),
    Quote(String),
    Code(String),
    Rule,
}

fn strip_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0;
    for c in line.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

pub fn markdown(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let mut code: Option<Vec<String>> = None;
    let flush = |para: &mut Vec<String>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Paragraph(para.join(" ")));
            para.clear();
        }
    };
    for raw in text.lines() {
        if let Some(lines) = &mut code {
            if raw.trim_start().starts_with("```") || raw.trim_start().starts_with("~~~") {
                out.push(Block::Code(lines.join("\n")));
                code = None;
            } else {
                lines.push(raw.to_string());
            }
            continue;
        }
        let line = raw.trim_end();
        let stripped;
        let line = if line.trim_start().starts_with('<') {
            stripped = strip_tags(line);
            if stripped.is_empty() {
                continue;
            }
            stripped.as_str()
        } else {
            line
        };
        let trimmed = line.trim_start();
        let indent = (line.len() - trimmed.len()) / 2;
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut para, &mut out);
            code = Some(Vec::new());
        } else if trimmed.is_empty() {
            flush(&mut para, &mut out);
        } else if let Some(level) =
            (1..=6).find(|n| trimmed.starts_with(&"#".repeat(*n)) && trimmed[*n..].starts_with(' '))
        {
            flush(&mut para, &mut out);
            out.push(Block::Heading(
                level as u8,
                trimmed[level + 1..]
                    .trim()
                    .trim_end_matches('#')
                    .trim()
                    .to_string(),
            ));
        } else if matches!(trimmed, "---" | "***" | "___") {
            flush(&mut para, &mut out);
            out.push(Block::Rule);
        } else if let Some(rest) = ["- ", "* ", "+ "]
            .iter()
            .find_map(|m| trimmed.strip_prefix(m))
        {
            flush(&mut para, &mut out);
            let rest = rest
                .strip_prefix("[ ] ")
                .map(|r| format!("\u{2610} {r}"))
                .or_else(|| rest.strip_prefix("[x] ").map(|r| format!("\u{2611} {r}")))
                .unwrap_or_else(|| rest.to_string());
            out.push(Block::Bullet(indent, rest));
        } else if let Some((num, rest)) = trimmed
            .split_once(". ")
            .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        {
            flush(&mut para, &mut out);
            out.push(Block::Numbered(num.parse().unwrap_or(1), rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix('>') {
            flush(&mut para, &mut out);
            out.push(Block::Quote(rest.trim().to_string()));
        } else {
            para.push(trimmed.to_string());
        }
    }
    if let Some(lines) = code {
        out.push(Block::Code(lines.join("\n")));
    }
    flush(&mut para, &mut out);
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    Plain,
    Bold,
    Italic,
    Code,
    Link,
}

pub fn inline(text: &str) -> Vec<(Span, String)> {
    let mut out: Vec<(Span, String)> = Vec::new();
    let mut rest = text;
    let mut plain = String::new();
    let flush = |plain: &mut String, out: &mut Vec<(Span, String)>| {
        if !plain.is_empty() {
            out.push((Span::Plain, std::mem::take(plain)));
        }
    };
    while let Some(c) = rest.chars().next() {
        let try_wrap = |rest: &str, marker: &str| -> Option<(String, usize)> {
            let inner = rest.strip_prefix(marker)?;
            let end = inner.find(marker)?;
            (end > 0).then(|| (inner[..end].to_string(), marker.len() * 2 + end))
        };
        if let Some((s, n)) = try_wrap(rest, "**").or_else(|| try_wrap(rest, "__")) {
            flush(&mut plain, &mut out);
            out.push((Span::Bold, s));
            rest = &rest[n..];
        } else if let Some((s, n)) = try_wrap(rest, "`") {
            flush(&mut plain, &mut out);
            out.push((Span::Code, s));
            rest = &rest[n..];
        } else if let Some((s, n)) = try_wrap(rest, "*")
            .or_else(|| try_wrap(rest, "_"))
            .filter(|(s, _)| !s.starts_with(' '))
        {
            flush(&mut plain, &mut out);
            out.push((Span::Italic, s));
            rest = &rest[n..];
        } else if c == '[' || rest.starts_with("![") {
            let skip = if c == '!' { 2 } else { 1 };
            let parsed = rest.find("](").and_then(|mid| {
                let close = rest[mid..].find(')')? + mid;
                let label = rest[skip..mid].to_string();
                let label = if skip == 2 {
                    format!("\u{1f5bc} {label}")
                } else {
                    label
                };
                Some((label, close + 1))
            });
            match parsed {
                Some((label, n)) => {
                    flush(&mut plain, &mut out);
                    out.push((Span::Link, label));
                    rest = &rest[n..];
                }
                None => {
                    plain.push(c);
                    rest = &rest[1..];
                }
            }
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    flush(&mut plain, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kinds() {
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path();
        assert_eq!(kind_of(&t.join("a.PNG")), Kind::Image);
        assert_eq!(kind_of(&t.join("a.rs")), Kind::Code(Lang::CLike));
        assert_eq!(kind_of(&t.join("README.md")), Kind::Markdown);
        assert_eq!(kind_of(&t.join("a.mp3")), Kind::Audio);
        assert_eq!(kind_of(&t.join("Makefile")), Kind::Text);
        std::fs::write(t.join("plain"), "hello\n").unwrap();
        std::fs::write(t.join("blob"), [0u8, 159, 146, 150]).unwrap();
        assert_eq!(kind_of(&t.join("plain")), Kind::Text);
        assert_eq!(kind_of(&t.join("blob")), Kind::Binary);
    }

    #[test]
    fn reads_truncated_text_and_hex() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("t");
        std::fs::write(&p, "a\r\nb\u{e9}cdefghijklmnopqrstuvwxyz").unwrap();
        let (text, cut) = read_text(&p, 5).unwrap();
        assert!(cut);
        assert_eq!(text, "a\nb");
        assert_eq!(read_text(&p, 6).unwrap().0, "a\nb\u{e9}");
        let (all, cut) = read_text(&p, 1000).unwrap();
        assert!(!cut && all.ends_with("xyz"));
        let lines = hex_dump(&p, 0, 20).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].hex.starts_with("61 0d 0a 62"));
        assert_eq!(lines[0].ascii.chars().next(), Some('a'));
        assert_eq!(lines[0].ascii.chars().nth(1), Some('.'));
        assert_eq!(lines[1].offset, 16);
        assert_eq!(hex_dump(&p, 1000, 16).unwrap().len(), 0);
    }

    #[test]
    fn highlights_code() {
        let mut block = false;
        let toks = highlight("let x = \"hi\"; // note", Lang::CLike, &mut block);
        assert_eq!(toks[0], (Tok::Keyword, "let".into()));
        assert!(toks.contains(&(Tok::String, "\"hi\"".into())));
        assert_eq!(toks.last().unwrap(), &(Tok::Comment, "// note".into()));
        let toks = highlight("a /* open", Lang::CLike, &mut block);
        assert!(block);
        assert_eq!(toks.last().unwrap().0, Tok::Comment);
        let toks = highlight("still */ 42", Lang::CLike, &mut block);
        assert!(!block);
        assert_eq!(toks.last().unwrap(), &(Tok::Number, "42".into()));
        let toks = highlight("name = \"oryn\"", Lang::Data, &mut false);
        assert_eq!(toks[0], (Tok::Type, "name".into()));
        let toks = highlight("SELECT id FROM t", Lang::Sql, &mut false);
        assert_eq!(toks[0].0, Tok::Keyword);
    }

    #[test]
    fn parses_markdown() {
        assert_eq!(
            markdown("<p align=\"center\">Hi <b>there</b></p>\n<br/>"),
            vec![Block::Paragraph("Hi there".into())]
        );
        assert_eq!(
            inline("![logo](a.png)"),
            vec![(Span::Link, "\u{1f5bc} logo".into())]
        );
        let md = "# Title\n\nSome **bold** and `code`.\nMore text.\n\n- one\n  - nested\n- [x] done\n1. first\n> quote\n\n```rust\nfn x() {}\n```\n---\n";
        let b = markdown(md);
        assert_eq!(b[0], Block::Heading(1, "Title".into()));
        assert_eq!(
            b[1],
            Block::Paragraph("Some **bold** and `code`. More text.".into())
        );
        assert_eq!(b[2], Block::Bullet(0, "one".into()));
        assert_eq!(b[3], Block::Bullet(1, "nested".into()));
        assert_eq!(b[4], Block::Bullet(0, "\u{2611} done".into()));
        assert_eq!(b[5], Block::Numbered(1, "first".into()));
        assert_eq!(b[6], Block::Quote("quote".into()));
        assert_eq!(b[7], Block::Code("fn x() {}".into()));
        assert_eq!(b[8], Block::Rule);
        let spans = inline("a **b** `c` *d* [e](http://x) f");
        assert_eq!(
            spans,
            vec![
                (Span::Plain, "a ".into()),
                (Span::Bold, "b".into()),
                (Span::Plain, " ".into()),
                (Span::Code, "c".into()),
                (Span::Plain, " ".into()),
                (Span::Italic, "d".into()),
                (Span::Plain, " ".into()),
                (Span::Link, "e".into()),
                (Span::Plain, " f".into()),
            ]
        );
    }
}
