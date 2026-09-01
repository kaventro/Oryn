use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;

use crate::services::ServiceResult;

/*
  Text extraction for the OOXML office formats — .docx, .xlsx, .pptx.

  All three are ZIP containers holding XML, so the `zip` dependency the archive
  code already pulls in is enough; nothing here needs a document library. What it
  does need is the character data of a handful of known elements, which is a much
  smaller job than parsing XML in general. The scanner below therefore tracks tag
  boundaries and attributes and nothing else — no namespaces, no DTDs, no
  validation. That is sound for these files specifically because they are machine
  generated and always well formed.

  The older .doc / .xls / .ppt formats are not OOXML. They are OLE compound
  binaries with an undocumented record layout, and reading them means a real
  library rather than a scanner. They are rejected by name so the viewer can say
  why instead of showing mojibake.
*/

const MAX_ENTRY_BYTES: u64 = 12 * 1024 * 1024;
const MAX_PARAGRAPHS: usize = 4000;
const MAX_ROWS: usize = 2000;
const MAX_COLS: usize = 64;
const MAX_SHEETS: usize = 24;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeSheet {
    pub name: String,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeDoc {
    pub kind: String,
    pub paragraphs: Vec<String>,
    pub sheets: Vec<OfficeSheet>,
    pub truncated: bool,
}

// ── Minimal XML scanning ──────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    /// `<name attrs>` — `attrs` is the raw run after the name.
    Open(&'a str, &'a str),
    /// `</name>`
    Close(&'a str),
    /// `<name attrs/>`
    Empty(&'a str, &'a str),
    /// Character data between tags, still entity-encoded.
    Text(&'a str),
}

pub fn scan(xml: &str) -> Vec<Node<'_>> {
    let bytes = xml.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            out.push(Node::Text(&xml[start..i]));
            continue;
        }

        // Comments, CDATA, processing instructions and declarations carry no
        // text we want; skip to the end of the construct.
        if xml[i..].starts_with("<!--") {
            i = find_from(xml, i, "-->").map_or(bytes.len(), |p| p + 3);
            continue;
        }
        if xml[i..].starts_with("<![CDATA[") {
            let start = i + 9;
            let end = find_from(xml, start, "]]>").unwrap_or(bytes.len());
            out.push(Node::Text(&xml[start..end.min(bytes.len())]));
            i = (end + 3).min(bytes.len());
            continue;
        }
        if xml[i..].starts_with("<?") || xml[i..].starts_with("<!") {
            i = find_from(xml, i, ">").map_or(bytes.len(), |p| p + 1);
            continue;
        }

        // A tag ends at the first '>' outside a quoted attribute value.
        let mut j = i + 1;
        let mut quote = 0u8;
        while j < bytes.len() {
            let c = bytes[j];
            if quote != 0 {
                if c == quote {
                    quote = 0;
                }
            } else if c == b'"' || c == b'\'' {
                quote = c;
            } else if c == b'>' {
                break;
            }
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }

        let inner = &xml[i + 1..j];
        i = j + 1;

        if let Some(name) = inner.strip_prefix('/') {
            out.push(Node::Close(name.trim()));
        } else if let Some(body) = inner.strip_suffix('/') {
            let (name, attrs) = split_name(body);
            out.push(Node::Empty(name, attrs));
        } else {
            let (name, attrs) = split_name(inner);
            out.push(Node::Open(name, attrs));
        }
    }

    out
}

fn find_from(hay: &str, from: usize, needle: &str) -> Option<usize> {
    hay.get(from..)
        .and_then(|s| s.find(needle))
        .map(|p| p + from)
}

fn split_name(inner: &str) -> (&str, &str) {
    let trimmed = inner.trim_start();
    match trimmed.find(|c: char| c.is_whitespace()) {
        Some(p) => (&trimmed[..p], trimmed[p..].trim()),
        None => (trimmed, ""),
    }
}

/// Reads one attribute out of the raw run captured by `scan`.
pub fn attr<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = attrs;
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].trim().trim_start_matches('/');
        let after = rest[eq + 1..].trim_start();
        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let value_start = quote.len_utf8();
        let end = after[value_start..].find(quote)? + value_start;
        if name == key {
            return Some(&after[value_start..end]);
        }
        rest = &after[end + quote.len_utf8()..];
    }
    None
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let semi = match tail.find(';') {
            // A bare '&' is not an entity; keep it and move on.
            Some(p) if (1..=12).contains(&p) => p,
            _ => {
                out.push('&');
                rest = &tail[1..];
                continue;
            }
        };
        let body = &tail[1..semi];
        let replacement = match body {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => body
                .strip_prefix('#')
                .and_then(|n| match n.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => n.parse::<u32>().ok(),
                })
                .and_then(char::from_u32),
        };
        match replacement {
            Some(c) => out.push(c),
            None => out.push_str(&tail[..=semi]),
        }
        rest = &tail[semi + 1..];
    }
    out.push_str(rest);
    out
}

// ── Container access ──────────────────────────────────────────────────────

fn read_entry(zip: &mut zip::ZipArchive<File>, name: &str) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    if entry.size() > MAX_ENTRY_BYTES {
        return None;
    }
    let mut buf = String::new();
    entry.read_to_string(&mut buf).ok()?;
    Some(buf)
}

// ── docx / pptx ───────────────────────────────────────────────────────────

/// Collects the text of each `para` element, joining every `text_el` inside
/// it. `<br/>` and `<tab/>` become whitespace so lines do not run together.
fn paragraphs_of(xml: &str, para: &str, text_el: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_text = false;

    for node in scan(xml) {
        match node {
            Node::Open(name, _) if name == para => {
                depth += 1;
                if depth == 1 {
                    current.clear();
                }
            }
            Node::Close(name) if name == para => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    out.push(current.trim_end().to_string());
                    if out.len() >= MAX_PARAGRAPHS {
                        return out;
                    }
                }
            }
            Node::Open(name, _) if name == text_el => in_text = true,
            Node::Close(name) if name == text_el => in_text = false,
            Node::Empty(name, _) if name.ends_with(":br") || name == "br" => current.push('\n'),
            Node::Empty(name, _) if name.ends_with(":tab") || name == "tab" => current.push('\t'),
            Node::Text(t) if in_text && depth > 0 => current.push_str(&decode_entities(t)),
            _ => {}
        }
    }

    out
}

// ── xlsx ──────────────────────────────────────────────────────────────────

/// Shared strings are stored once per workbook; a cell of type "s" holds an
/// index into this table rather than its own text.
fn shared_strings(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_item = false;
    let mut in_text = false;

    for node in scan(xml) {
        match node {
            Node::Open("si", _) => {
                in_item = true;
                current.clear();
            }
            Node::Close("si") => {
                in_item = false;
                out.push(std::mem::take(&mut current));
            }
            Node::Open("t", _) => in_text = true,
            Node::Close("t") => in_text = false,
            Node::Text(t) if in_item && in_text => current.push_str(&decode_entities(t)),
            _ => {}
        }
    }

    out
}

/// "AB12" -> 27. Excel omits empty cells entirely, so the column letters are the
/// only way to know where a value belongs in its row.
fn column_index(reference: &str) -> usize {
    let mut n = 0usize;
    for c in reference.chars() {
        match c.to_ascii_uppercase() {
            'A'..='Z' => n = n * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1),
            _ => break,
        }
    }
    n.saturating_sub(1)
}

fn sheet_rows(xml: &str, shared: &[String]) -> (Vec<Vec<String>>, bool) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut col = 0usize;
    let mut cell_type = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut truncated = false;

    for node in scan(xml) {
        match node {
            Node::Open("row", _) => {
                row.clear();
            }
            Node::Close("row") => {
                while row.last().is_some_and(|c| c.is_empty()) {
                    row.pop();
                }
                rows.push(std::mem::take(&mut row));
                if rows.len() >= MAX_ROWS {
                    truncated = true;
                    return (rows, truncated);
                }
            }
            Node::Open("c", attrs) | Node::Empty("c", attrs) => {
                col = attr(attrs, "r")
                    .map(column_index)
                    .unwrap_or_else(|| row.len());
                cell_type = attr(attrs, "t").unwrap_or("").to_string();
                value.clear();
            }
            Node::Open("v", _) | Node::Open("t", _) => in_value = true,
            Node::Close("v") | Node::Close("t") => in_value = false,
            Node::Text(t) if in_value => value.push_str(&decode_entities(t)),
            Node::Close("c") => {
                let text = if cell_type == "s" {
                    value
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| shared.get(i).cloned())
                        .unwrap_or_default()
                } else {
                    value.clone()
                };
                if col < MAX_COLS {
                    if row.len() <= col {
                        row.resize(col + 1, String::new());
                    }
                    row[col] = text;
                } else {
                    truncated = true;
                }
                value.clear();
            }
            _ => {}
        }
    }

    if !row.is_empty() {
        rows.push(row);
    }
    (rows, truncated)
}

/// Sheet display names live in workbook.xml keyed by relationship id, and the
/// id-to-file mapping lives in the rels part. Resolving both is what keeps a
/// tab labelled "Q3" from being shown as "sheet2.xml".
fn sheet_targets(workbook: &str, rels: &str) -> Vec<(String, String)> {
    let mut by_id: BTreeMap<String, String> = BTreeMap::new();
    for node in scan(rels) {
        if let Node::Open(name, attrs) | Node::Empty(name, attrs) = node {
            if name == "Relationship" {
                if let (Some(id), Some(target)) = (attr(attrs, "Id"), attr(attrs, "Target")) {
                    by_id.insert(id.to_string(), target.trim_start_matches('/').to_string());
                }
            }
        }
    }

    let mut out = Vec::new();
    for node in scan(workbook) {
        if let Node::Open(name, attrs) | Node::Empty(name, attrs) = node {
            if name == "sheet" {
                let label = attr(attrs, "name").map(decode_entities).unwrap_or_default();
                let target = attr(attrs, "r:id")
                    .or_else(|| attr(attrs, "id"))
                    .and_then(|id| by_id.get(id))
                    .cloned();
                if let Some(t) = target {
                    let path = if t.starts_with("xl/") {
                        t
                    } else {
                        format!("xl/{t}")
                    };
                    out.push((label, path));
                }
            }
        }
    }
    out
}

// ── Entry point ───────────────────────────────────────────────────────────

pub fn extension_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn read_office(path: &str) -> ServiceResult<OfficeDoc> {
    let ext = extension_of(path);

    if matches!(ext.as_str(), "doc" | "xls" | "ppt") {
        anyhow::bail!(
            "{} is the pre-2007 binary Office format, not OOXML. Open it in its \
             application, or save it as .{}x to preview it here.",
            ext.to_uppercase(),
            ext
        );
    }

    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| anyhow::anyhow!("not a readable Office container: {e}"))?;

    match ext.as_str() {
        "docx" => {
            let xml = read_entry(&mut zip, "word/document.xml")
                .ok_or_else(|| anyhow::anyhow!("word/document.xml is missing or too large"))?;
            let paragraphs = paragraphs_of(&xml, "w:p", "w:t");
            let truncated = paragraphs.len() >= MAX_PARAGRAPHS;
            Ok(OfficeDoc {
                kind: "docx".into(),
                paragraphs,
                sheets: Vec::new(),
                truncated,
            })
        }
        "pptx" => {
            let mut paragraphs = Vec::new();
            let mut slides: Vec<String> = zip
                .file_names()
                .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
                .map(|n| n.to_string())
                .collect();
            slides.sort_by_key(|n| slide_number(n));

            for (index, name) in slides.iter().enumerate() {
                if let Some(xml) = read_entry(&mut zip, name) {
                    paragraphs.push(format!("--- Slide {} ---", index + 1));
                    paragraphs.extend(paragraphs_of(&xml, "a:p", "a:t"));
                }
                if paragraphs.len() >= MAX_PARAGRAPHS {
                    break;
                }
            }
            let truncated = paragraphs.len() >= MAX_PARAGRAPHS;
            Ok(OfficeDoc {
                kind: "pptx".into(),
                paragraphs,
                sheets: Vec::new(),
                truncated,
            })
        }
        "xlsx" | "xlsm" => {
            let shared = read_entry(&mut zip, "xl/sharedStrings.xml")
                .map(|x| shared_strings(&x))
                .unwrap_or_default();
            let workbook = read_entry(&mut zip, "xl/workbook.xml").unwrap_or_default();
            let rels = read_entry(&mut zip, "xl/_rels/workbook.xml.rels").unwrap_or_default();

            let mut targets = sheet_targets(&workbook, &rels);
            if targets.is_empty() {
                // No usable workbook metadata; fall back to whatever sheet parts
                // the container holds so the file still previews.
                let mut names: Vec<String> = zip
                    .file_names()
                    .filter(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
                    .map(|n| n.to_string())
                    .collect();
                names.sort_by_key(|n| slide_number(n));
                targets = names
                    .into_iter()
                    .enumerate()
                    .map(|(i, n)| (format!("Sheet{}", i + 1), n))
                    .collect();
            }

            let mut sheets = Vec::new();
            let mut truncated = targets.len() > MAX_SHEETS;
            for (label, target) in targets.into_iter().take(MAX_SHEETS) {
                let Some(xml) = read_entry(&mut zip, &target) else {
                    continue;
                };
                let (rows, cut) = sheet_rows(&xml, &shared);
                truncated |= cut;
                sheets.push(OfficeSheet {
                    name: label,
                    rows,
                    truncated: cut,
                });
            }

            Ok(OfficeDoc {
                kind: "xlsx".into(),
                paragraphs: Vec::new(),
                sheets,
                truncated,
            })
        }
        other => anyhow::bail!("no Office reader for .{other}"),
    }
}

/// Sorts sheet2 before sheet10, which a plain string sort would not.
fn slide_number(name: &str) -> u32 {
    name.chars()
        .rev()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_splits_tags_from_text() {
        let nodes = scan("<a x=\"1\">hi</a><b/>");
        assert_eq!(nodes[0], Node::Open("a", "x=\"1\""));
        assert_eq!(nodes[1], Node::Text("hi"));
        assert_eq!(nodes[2], Node::Close("a"));
        assert_eq!(nodes[3], Node::Empty("b", ""));
    }

    #[test]
    fn scan_ignores_a_gt_inside_an_attribute_value() {
        let nodes = scan("<a t=\"x>y\">z</a>");
        assert_eq!(nodes[0], Node::Open("a", "t=\"x>y\""));
        assert_eq!(nodes[1], Node::Text("z"));
    }

    #[test]
    fn scan_skips_comments_and_declarations_but_keeps_cdata() {
        let nodes = scan("<?xml version=\"1.0\"?><!-- note --><a><![CDATA[raw<>]]></a>");
        assert_eq!(nodes[0], Node::Open("a", ""));
        assert_eq!(nodes[1], Node::Text("raw<>"));
    }

    #[test]
    fn attr_reads_values_and_ignores_prefixes_of_other_keys() {
        assert_eq!(attr("r=\"A1\" t=\"s\"", "t"), Some("s"));
        assert_eq!(attr("rid=\"9\" r=\"A1\"", "r"), Some("A1"));
        assert_eq!(attr("r='B2'", "r"), Some("B2"));
        assert_eq!(attr("r=\"A1\"", "missing"), None);
    }

    #[test]
    fn decode_entities_handles_named_and_numeric_forms() {
        assert_eq!(decode_entities("a &amp; b"), "a & b");
        assert_eq!(decode_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_entities("&quot;q&apos;"), "\"q'");
        assert_eq!(decode_entities("&#65;&#x42;"), "AB");
        // A bare ampersand is not an entity and must survive untouched.
        assert_eq!(decode_entities("Tom & Jerry"), "Tom & Jerry");
        assert_eq!(decode_entities("&unknown;"), "&unknown;");
    }

    #[test]
    fn paragraphs_join_runs_and_split_on_paragraph_boundaries() {
        let xml = "<w:body><w:p><w:r><w:t>Hello </w:t></w:r>\
                   <w:r><w:t>world</w:t></w:r></w:p>\
                   <w:p><w:r><w:t>Second</w:t></w:r></w:p></w:body>";
        assert_eq!(
            paragraphs_of(xml, "w:p", "w:t"),
            vec!["Hello world", "Second"]
        );
    }

    #[test]
    fn paragraph_text_outside_a_run_is_not_collected() {
        // Numbering and style parts sit between runs; their content is not body text.
        let xml = "<w:p><w:pPr>styling</w:pPr><w:r><w:t>only this</w:t></w:r></w:p>";
        assert_eq!(paragraphs_of(xml, "w:p", "w:t"), vec!["only this"]);
    }

    #[test]
    fn column_index_maps_letters_to_positions() {
        assert_eq!(column_index("A1"), 0);
        assert_eq!(column_index("B2"), 1);
        assert_eq!(column_index("Z9"), 25);
        assert_eq!(column_index("AA1"), 26);
        assert_eq!(column_index("AB12"), 27);
    }

    #[test]
    fn shared_string_cells_resolve_and_gaps_are_filled() {
        let shared = vec!["Name".to_string(), "Total".to_string()];
        // Column B is absent from the row; C must still land in position 2.
        let xml = "<sheetData><row><c r=\"A1\" t=\"s\"><v>0</v></c>\
                   <c r=\"C1\" t=\"s\"><v>1</v></c></row>\
                   <row><c r=\"A2\"><v>42</v></c></row></sheetData>";
        let (rows, truncated) = sheet_rows(xml, &shared);
        assert!(!truncated);
        assert_eq!(rows[0], vec!["Name", "", "Total"]);
        assert_eq!(rows[1], vec!["42"]);
    }

    #[test]
    fn inline_string_cells_are_read_from_their_own_text() {
        let xml = "<row><c r=\"A1\" t=\"inlineStr\"><is><t>Inline</t></is></c></row>";
        let (rows, _) = sheet_rows(xml, &[]);
        assert_eq!(rows[0], vec!["Inline"]);
    }

    #[test]
    fn sheet_targets_pair_names_with_their_parts_through_the_rels() {
        let workbook = "<workbook><sheets><sheet name=\"Q3\" sheetId=\"1\" r:id=\"rId7\"/>\
                        <sheet name=\"Notes\" sheetId=\"2\" r:id=\"rId3\"/></sheets></workbook>";
        let rels = "<Relationships>\
                    <Relationship Id=\"rId3\" Target=\"worksheets/sheet2.xml\"/>\
                    <Relationship Id=\"rId7\" Target=\"worksheets/sheet1.xml\"/></Relationships>";
        assert_eq!(
            sheet_targets(workbook, rels),
            vec![
                ("Q3".to_string(), "xl/worksheets/sheet1.xml".to_string()),
                ("Notes".to_string(), "xl/worksheets/sheet2.xml".to_string()),
            ]
        );
    }

    #[test]
    fn slide_number_orders_numerically_not_lexically() {
        let mut names = vec!["ppt/slides/slide10.xml", "ppt/slides/slide2.xml"];
        names.sort_by_key(|n| slide_number(n));
        assert_eq!(
            names,
            vec!["ppt/slides/slide2.xml", "ppt/slides/slide10.xml"]
        );
    }

    #[test]
    fn legacy_binary_formats_are_refused_by_name() {
        let err = read_office("C:/tmp/report.doc").unwrap_err().to_string();
        assert!(err.contains("binary Office format"), "got: {err}");
        assert!(
            err.contains(".docx"),
            "the message should name the fix: {err}"
        );
    }
}
