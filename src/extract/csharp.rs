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
    let mut depth: usize = 0;
    // (depth_at_open, fully_qualified_name) — namespaces and classes
    let mut scope_stack: Vec<(usize, String)> = Vec::new();
    // class/namespace declared but `{` not yet seen (Allman-brace style)
    let mut pending_scope: Option<String> = None;

    while i < lines.len() {
        let t = lines[i].trim();
        let opens = t.chars().filter(|&c| c == '{').count();
        let closes = t.chars().filter(|&c| c == '}').count();

        // Process closes first
        if closes > 0 {
            let new_depth = depth.saturating_sub(closes);
            scope_stack.retain(|(d, _)| *d < new_depth);
            depth = new_depth;
        }

        // If this line opens braces, commit any pending scope and update depth
        if opens > 0 {
            if let Some(name) = pending_scope.take() {
                scope_stack.push((depth, name));
            }
            depth += opens;
        } else {
            pending_scope = None;
        }

        if t.starts_with("///") {
            let (raw, end) = collect_xml_doc(&lines, i);
            let block = strip_xml_tags(raw);
            let sym = next_non_attr(&lines, end + 1);
            let (raw_name, kind) = detect_cs_symbol(sym);
            if !raw_name.is_empty() {
                let name = cs_qualify(&raw_name, &scope_stack);
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::CSharp, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }

        // Track scope-opening declarations (namespace / class / struct / interface)
        if let Some(decl) = detect_cs_scope_decl(t) {
            let full = if let Some((_, parent)) = scope_stack.last() {
                format!("{parent}.{decl}")
            } else {
                decl
            };
            if opens > 0 {
                // `{` already handled above; push with the depth *before* the opens
                scope_stack.push((depth - opens, full));
            } else {
                pending_scope = Some(full);
            }
        }

        i += 1;
    }
    items
}

/// Qualify `name` with the innermost scope (namespace / class).
fn cs_qualify(name: &str, scope_stack: &[(usize, String)]) -> String {
    match scope_stack.last() {
        Some((_, parent)) => format!("{parent}.{name}"),
        None => name.to_owned(),
    }
}

/// Detect a namespace / class / struct / interface declaration line and return its simple name.
fn detect_cs_scope_decl(line: &str) -> Option<String> {
    let keywords = [
        "public", "private", "protected", "internal", "static",
        "abstract", "sealed", "partial", "readonly",
    ];
    let mut words: Vec<&str> = line.split_whitespace().collect();
    words.retain(|w| !keywords.contains(w));
    let clean = words.join(" ");
    let clean = clean.trim();
    for kw in ["namespace ", "class ", "struct ", "interface "] {
        if let Some(rest) = clean.strip_prefix(kw) {
            let name = first_ident(rest.split(':').next().unwrap_or(rest).split('<').next().unwrap_or(rest));
            if !name.is_empty() { return Some(name); }
        }
    }
    None
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
    // Method: the identifier directly before the first `(`
    if let Some(paren) = line.find('(') {
        let before = line[..paren].trim_end();
        if let Some(tok) = before.split_whitespace().last() {
            let name = tok.split('<').next().unwrap_or(tok).trim_end_matches(';');
            if !name.is_empty() && name.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
                return name.to_string();
            }
        }
    }
    // Property: the identifier directly before the first `{`
    if let Some(brace) = line.find('{') {
        let before = line[..brace].trim_end();
        if let Some(tok) = before.split_whitespace().last() {
            let name = tok.split('<').next().unwrap_or(tok).trim_end_matches(';');
            if !name.is_empty() && name.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
                return name.to_string();
            }
        }
    }
    String::new()
}
