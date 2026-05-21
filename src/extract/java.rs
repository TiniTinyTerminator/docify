use std::path::Path;
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, collect_c_block, next_decl_sym, first_ident, func_name_before_paren};

pub struct JavaExtractor;

impl DocExtractor for JavaExtractor {
    fn extensions(&self) -> &[&str] { &["java"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_java(src, path)
    }
}

pub(super) fn extract_java(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();

        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_decl_sym(&lines, end + 1);
            let (name, kind) = detect_java_symbol(sym);
            if !name.is_empty() || kind != DocKind::Unknown {
                let item = build_item(block, name, kind, file, i + 1, DocLanguage::Java, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }

        i += 1;
    }
    items
}

fn detect_java_symbol(line: &str) -> (String, DocKind) {
    const MODIFIERS: &[&str] = &[
        "public ", "protected ", "private ", "static ", "final ",
        "abstract ", "synchronized ", "native ", "transient ", "volatile ",
        "default ", "strictfp ", "sealed ", "non-sealed ",
    ];
    let mut t = line.trim();
    'outer: loop {
        // Skip annotation lines like @Override, @Deprecated, @SuppressWarnings(...)
        if t.starts_with('@') {
            if let Some(rest) = t.split_once(char::is_whitespace) {
                t = rest.1.trim_start();
                continue 'outer;
            }
        }
        for m in MODIFIERS {
            if let Some(rest) = t.strip_prefix(m) { t = rest.trim_start(); continue 'outer; }
        }
        break;
    }

    if let Some(r) = t.strip_prefix("class ")      { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("interface ")  { return (first_ident(r), DocKind::Interface); }
    if let Some(r) = t.strip_prefix("enum ")       { return (first_ident(r), DocKind::Enum); }
    if let Some(r) = t.strip_prefix("record ")     { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("@interface ") { return (first_ident(r), DocKind::Interface); }

    if let Some(name) = func_name_before_paren(t) {
        return (name, DocKind::Function);
    }

    (String::new(), DocKind::Unknown)
}
