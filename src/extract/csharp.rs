use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct CSharpExtractor;

impl DocExtractor for CSharpExtractor {
    fn extensions(&self) -> &[&str] { &["cs"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_csharp(src, path)
    }
}

fn extract_csharp(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("///") {
            let (raw, end) = collect_xml_doc(&lines, i);
            let block = strip_xml_tags(raw);
            let sym = next_non_attr(&lines, end + 1);
            let (name, kind) = detect_cs_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::CSharp, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn collect_xml_doc(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("///") {
            out.push(t[3..].trim().to_string());
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

/// Convert XML doc comment lines to @tag equivalents that `build_item` understands.
fn strip_xml_tags(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let t = line.trim();
        if t.starts_with("<summary>") || t.starts_with("<summary/>") {
            let inner = t.trim_start_matches("<summary>")
                .trim_end_matches("</summary>").trim();
            if !inner.is_empty() { out.push(inner.to_string()); }
        } else if t == "</summary>" || t == "<summary>" {
            // multi-line summary — content lines are already pushed
        } else if let Some(rest) = t.strip_prefix("<param name=\"") {
            let name_end = rest.find('"').unwrap_or(rest.len());
            let name = &rest[..name_end];
            let desc = rest.get(name_end + 2..)
                .and_then(|r| r.strip_suffix("</param>"))
                .map(str::trim)
                .unwrap_or("");
            out.push(format!("@param {name} {desc}"));
        } else if let Some(rest) = t.strip_prefix("<returns>") {
            let desc = rest.strip_suffix("</returns>").unwrap_or(rest).trim();
            out.push(format!("@return {desc}"));
        } else if let Some(rest) = t.strip_prefix("<exception cref=\"") {
            let end = rest.find('"').unwrap_or(rest.len());
            let exc = &rest[..end];
            let desc = rest.get(end + 2..)
                .and_then(|r| r.strip_suffix("</exception>"))
                .map(str::trim)
                .unwrap_or("");
            out.push(format!("@throws {exc} {desc}"));
        } else if let Some(rest) = t.strip_prefix("<remarks>") {
            let inner = rest.strip_suffix("</remarks>").unwrap_or(rest).trim();
            if !inner.is_empty() { out.push(inner.to_string()); }
        } else if let Some(rest) = t.strip_prefix("<see cref=\"") {
            let end = rest.find('"').unwrap_or(rest.len());
            let target = &rest[..end];
            out.push(format!("@see {target}"));
        } else {
            // strip remaining XML tags; keep plain text
            let plain = strip_inline_tags(t);
            if !plain.is_empty() { out.push(plain); }
        }
    }
    out
}

fn strip_inline_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Skip `[Attribute]` lines and blank lines to reach the actual declaration.
fn next_non_attr<'a>(lines: &[&'a str], from: usize) -> &'a str {
    let mut i = from;
    loop {
        let Some(&l) = lines.get(i) else { return "" };
        let t = l.trim();
        if t.is_empty() || t.starts_with('[') {
            i += 1;
            continue;
        }
        return t;
    }
}

fn detect_cs_symbol(line: &str) -> (String, DocKind) {
    // Strip access modifiers and other keywords
    let keywords = ["public", "private", "protected", "internal", "static",
        "abstract", "virtual", "override", "sealed", "partial", "readonly",
        "async", "extern", "new", "unsafe"];
    let mut words: Vec<&str> = line.split_whitespace().collect();
    words.retain(|w| !keywords.contains(w));

    let t = words.join(" ");
    let t = t.trim();

    if let Some(r) = t.strip_prefix("class ") { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("struct ") { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("interface ") { return (first_ident(r), DocKind::Interface); }
    if let Some(r) = t.strip_prefix("enum ") { return (first_ident(r), DocKind::Enum); }
    if let Some(r) = t.strip_prefix("record ") { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("delegate ") {
        // delegate ReturnType Name(...)
        let name = r.split_whitespace()
            .find(|w| w.contains('('))
            .and_then(|w| w.split('(').next())
            .or_else(|| r.split_whitespace().nth(1))
            .map(|s| s.trim_end_matches('(').to_string())
            .unwrap_or_default();
        return (name, DocKind::Typedef);
    }
    if let Some(r) = t.strip_prefix("namespace ") { return (first_ident(r), DocKind::Module); }
    if let Some(r) = t.strip_prefix("event ") {
        // event EventHandler Name
        let parts: Vec<_> = r.split_whitespace().collect();
        let name = parts.get(1).copied().unwrap_or("").split('<').next().unwrap_or("");
        return (name.to_string(), DocKind::Variable);
    }

    // Method or property: ReturnType Name( or ReturnType Name {
    let name = detect_cs_member(t);
    if !name.is_empty() {
        let is_prop = t.contains('{') && !t.contains('(');
        let kind = if is_prop { DocKind::Variable } else { DocKind::Function };
        return (name, kind);
    }

    (String::new(), DocKind::Unknown)
}

fn detect_cs_member(line: &str) -> String {
    // Expect "ReturnType Name(" or "ReturnType Name<T>(" or property "RetType Name {"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 { return String::new(); }
    // The member name is the token just before a `(` or `{` or `<`
    for w in &parts[1..] {
        let base = w.split(|c| c == '(' || c == '<' || c == '{').next().unwrap_or(w);
        let base = base.trim_end_matches(';');
        if !base.is_empty() && base.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
            if w.contains('(') || w.contains('{') || parts.iter().any(|p| p.contains('(')) {
                return base.to_string();
            }
        }
    }
    String::new()
}
