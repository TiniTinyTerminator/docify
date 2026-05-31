use super::common::{build_item, collect_c_block, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct PhpExtractor;

impl DocExtractor for PhpExtractor {
    fn extensions(&self) -> &[&str] { &["php"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_php(src, path)
    }
}

fn extract_php(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_non_attr(&lines, end + 1);
            let (name, kind) = detect_php_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::Php, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn next_non_attr<'a>(lines: &[&'a str], from: usize) -> &'a str {
    let mut i = from;
    loop {
        let Some(&l) = lines.get(i) else { return "" };
        let t = l.trim();
        if t.is_empty() || t.starts_with("#[") || t.starts_with("<<") { i += 1; continue; }
        return t;
    }
}

fn detect_php_symbol(line: &str) -> (String, DocKind) {
    let mods = ["public", "protected", "private", "static", "abstract",
        "final", "readonly", "async"];
    let mut t = line.trim();
    loop {
        let mut advanced = false;
        for m in &mods {
            if let Some(r) = t.strip_prefix(m) {
                t = r.trim_start(); advanced = true; break;
            }
        }
        if !advanced { break; }
    }
    if let Some(r) = t.strip_prefix("function ") {
        // Strip & for by-reference return
        let r = r.trim_start_matches('&');
        let name = r.split('(').next().unwrap_or("").trim().to_string();
        return (name, DocKind::Function);
    }
    if let Some(r) = t.strip_prefix("class ") { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("abstract class ") { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("interface ") { return (first_ident(r), DocKind::Interface); }
    if let Some(r) = t.strip_prefix("trait ") { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("enum ") { return (first_ident(r), DocKind::Enum); }
    // $variable declaration or const
    if let Some(r) = t.strip_prefix("const ") { return (first_ident(r), DocKind::Variable); }
    if t.starts_with('$') {
        let name = first_ident(&t[1..]);
        if !name.is_empty() { return (format!("${name}"), DocKind::Variable); }
    }
    (String::new(), DocKind::Unknown)
}
