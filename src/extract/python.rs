use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct PythonExtractor;

impl DocExtractor for PythonExtractor {
    fn extensions(&self) -> &[&str] { &["py", "pyi"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_python(src, path)
    }
}

fn extract_python(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    // Track class scope by indentation for name qualification.
    let mut class_stack: Vec<(usize, String)> = Vec::new(); // (indent, name)

    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim();
        let indent = raw.len() - raw.trim_start().len();

        // Pop classes whose indent is >= current non-blank line indent.
        if !t.is_empty() {
            class_stack.retain(|&(ci, _)| ci < indent);
        }

        if t.is_empty() { i += 1; continue; }

        // Detect `def` / `async def` / `class`
        let stripped = t.strip_prefix("async ").unwrap_or(t);
        if let Some((name, kind)) = detect_python_def(stripped) {
            // Look for docstring on next non-blank line
            let decl_line = i + 1;
            let body_start = skip_to_body(&lines, i);
            if let Some((doc_lines, end)) = collect_docstring(&lines, body_start) {
                let scope = class_stack.last().map(|(_, n)| n.as_str()).unwrap_or("");
                let qualified = if !scope.is_empty() && kind != DocKind::Class {
                    format!("{scope}.{name}")
                } else {
                    name.clone()
                };
                let block = parse_python_docstring(doc_lines);
                let item = build_item(block, qualified, kind.clone(), file,
                    decl_line, DocLanguage::Python, t.to_string());
                if item_has_content(&item) { items.push(item); }
                i = end + 1;
            } else {
                i += 1;
            }
            if kind == DocKind::Class {
                class_stack.push((indent, name));
            }
            continue;
        }

        // Module-level variable with inline comment `#:` (Sphinx autodoc style)
        if let Some(rest) = t.strip_prefix("#:") {
            let brief = rest.trim().to_string();
            let sym = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
            let name = detect_python_assignment(sym);
            if !name.is_empty() {
                let item = build_item(vec![brief], name, DocKind::Variable,
                    file, i + 1, DocLanguage::Python, sym.to_string());
                if item_has_content(&item) { items.push(item); }
                i += 2;
                continue;
            }
        }

        i += 1;
    }
    items
}

fn detect_python_def(t: &str) -> Option<(String, DocKind)> {
    if let Some(r) = t.strip_prefix("def ") {
        let name = r.split('(').next().unwrap_or("").trim().to_string();
        if !name.is_empty() { return Some((name, DocKind::Function)); }
    }
    if let Some(r) = t.strip_prefix("class ") {
        let name = first_ident(r.split('(').next().unwrap_or(r).split(':').next().unwrap_or(r));
        if !name.is_empty() { return Some((name, DocKind::Class)); }
    }
    None
}

fn detect_python_assignment(t: &str) -> String {
    if let Some((lhs, _)) = t.split_once('=') {
        let name = lhs.trim();
        if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() {
            return name.to_string();
        }
    }
    String::new()
}

/// Find the first line of the function/class body (after the `:` header, possibly multi-line).
fn skip_to_body(lines: &[&str], def_line: usize) -> usize {
    let mut i = def_line;
    loop {
        let Some(&l) = lines.get(i) else { return i };
        if l.trim().ends_with(':') || l.contains("):") || l.contains("->") && l.trim_end().ends_with(':') {
            return i + 1;
        }
        i += 1;
        if i > def_line + 5 { return def_line + 1; }
    }
}

/// Collect a `"""..."""` or `'''...'''` docstring starting at `from`.
fn collect_docstring(lines: &[&str], from: usize) -> Option<(Vec<String>, usize)> {
    let mut i = from;
    // Skip blank lines and decorator-like lines
    while i < lines.len() && lines[i].trim().is_empty() { i += 1; }
    let Some(&first) = lines.get(i) else { return None };
    let t = first.trim();
    let quote = if t.starts_with("\"\"\"") { "\"\"\"" }
        else if t.starts_with("'''") { "'''" }
        else { return None };

    let content = &t[quote.len()..];
    // Single-line docstring: `"""text"""`
    if let Some(inner) = content.strip_suffix(quote) {
        return Some((vec![inner.trim().to_string()], i));
    }
    // Multi-line: collect until closing quotes
    let mut out = vec![content.to_string()];
    i += 1;
    while i < lines.len() {
        let l = lines[i].trim();
        if let Some(before) = l.find(quote).map(|p| &l[..p]) {
            out.push(before.to_string());
            return Some((out, i));
        }
        out.push(l.to_string());
        i += 1;
    }
    None
}

/// Convert Python docstring lines (Google/NumPy/reStructuredText style) to `@tag` lines.
fn parse_python_docstring(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section: Option<&'static str> = None;

    for line in &lines {
        let t = line.trim();

        // Detect Google-style section headers: "Args:", "Returns:", etc.
        if let Some(tag) = google_section(t) {
            in_section = Some(tag);
            continue;
        }

        // Google-style parameter inside Args/Raises section: "    name (type): desc"
        if matches!(in_section, Some("@param") | Some("@raises")) {
            if let Some(param) = parse_google_param(t) {
                out.push(format!("{} {param}", in_section.unwrap()));
                continue;
            }
        }
        if in_section == Some("@return") && !t.is_empty() {
            out.push(format!("@return {t}"));
            in_section = None;
            continue;
        }

        // NumPy-style underline (e.g. "------") — skip
        if t.chars().all(|c| c == '-' || c == '=') && !t.is_empty() { continue; }

        // reST-style :param x: / :type x: / :returns: / :raises X:
        if let Some(rest) = t.strip_prefix(":param ") {
            if let Some((name, desc)) = rest.split_once(':') {
                out.push(format!("@param {} {}", name.trim(), desc.trim()));
                continue;
            }
        }
        if let Some(rest) = t.strip_prefix(":returns:").or_else(|| t.strip_prefix(":return:")) {
            out.push(format!("@return {}", rest.trim()));
            continue;
        }
        if let Some(rest) = t.strip_prefix(":raises ").or_else(|| t.strip_prefix(":raise ")) {
            if let Some((exc, desc)) = rest.split_once(':') {
                out.push(format!("@throws {} {}", exc.trim(), desc.trim()));
                continue;
            }
        }
        if t.starts_with(":type ") || t.starts_with(":rtype:") { continue; }

        in_section = None;
        out.push(line.clone());
    }
    out
}

fn google_section(t: &str) -> Option<&'static str> {
    match t {
        "Args:" | "Arguments:" | "Parameters:" => Some("@param"),
        "Returns:" | "Return:" => Some("@return"),
        "Raises:" | "Raise:" | "Throws:" => Some("@raises"),
        "Note:" | "Notes:" => Some("@note"),
        "Warning:" | "Warnings:" => Some("@warning"),
        "Example:" | "Examples:" => Some("@example"),
        "See Also:" => Some("@see"),
        _ => None,
    }
}

fn parse_google_param(t: &str) -> Option<String> {
    // "    name (type): description"  or  "    name: description"
    let t = t.trim_start();
    if t.starts_with('-') || t.is_empty() { return None; }
    let (name_part, desc) = if let Some(colon) = t.find(':') {
        (&t[..colon], t[colon + 1..].trim())
    } else { return None; };
    // Strip optional "(type)" from name_part
    let name = name_part.split('(').next().unwrap_or(name_part).trim();
    if name.is_empty() || name.contains(' ') { return None; }
    Some(format!("{name} {desc}"))
}
