use super::common::{build_item, collect_c_block, collect_line_block, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct TypeScriptExtractor;
pub struct JavaScriptExtractor;

impl DocExtractor for TypeScriptExtractor {
    fn extensions(&self) -> &[&str] { &["ts", "tsx"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_js(src, path, DocLanguage::TypeScript)
    }
}
impl DocExtractor for JavaScriptExtractor {
    fn extensions(&self) -> &[&str] { &["js", "jsx", "mjs", "cjs"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_js(src, path, DocLanguage::JavaScript)
    }
}

fn extract_js(src: &str, file: &Path, lang: DocLanguage) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_decl(&lines, end + 1);
            let (name, kind) = detect_js_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2, lang.clone(), sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        if t.starts_with("///") {
            let (block, end) = collect_line_block(&lines, i, "///");
            let sym = next_decl(&lines, end + 1);
            let (name, kind) = detect_js_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2, lang.clone(), sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn next_decl<'a>(lines: &[&'a str], from: usize) -> &'a str {
    lines.get(from).map(|l| l.trim()).unwrap_or("")
}

fn detect_js_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();
    // export / export default
    let t = t.strip_prefix("export").map(|r| r.trim()).unwrap_or(t);
    let t = t.strip_prefix("default").map(|r| r.trim()).unwrap_or(t);
    // async
    let t = t.strip_prefix("async").map(|r| r.trim()).unwrap_or(t);
    // declare
    let t = t.strip_prefix("declare").map(|r| r.trim()).unwrap_or(t);

    if let Some(r) = t.strip_prefix("function ").or_else(|| t.strip_prefix("function*")) {
        return (first_ident(r.trim_start_matches('*').trim()), DocKind::Function);
    }
    if let Some(r) = t.strip_prefix("class ") {
        return (first_ident(r), DocKind::Class);
    }
    if let Some(r) = t.strip_prefix("interface ") {
        return (first_ident(r), DocKind::Interface);
    }
    if let Some(r) = t.strip_prefix("type ") {
        return (first_ident(r), DocKind::Typedef);
    }
    if let Some(r) = t.strip_prefix("enum ") {
        return (first_ident(r), DocKind::Enum);
    }
    if let Some(r) = t.strip_prefix("namespace ").or_else(|| t.strip_prefix("module ")) {
        return (first_ident(r), DocKind::Module);
    }
    // const/let/var name = function / arrow
    for kw in ["const ", "let ", "var "] {
        if let Some(r) = t.strip_prefix(kw) {
            let name = first_ident(r);
            if !name.is_empty() {
                let after = r[name.len()..].trim_start().trim_start_matches(':');
                let after = after.trim_start();
                // skip generic type parameters
                let after = if after.starts_with('<') {
                    after.find('>').map(|p| after[p + 1..].trim_start()).unwrap_or(after)
                } else { after };
                let after = after.trim_start_matches('=').trim();
                if after.starts_with("function") || after.starts_with("async") || after.starts_with('(') {
                    return (name, DocKind::Function);
                }
                return (name, DocKind::Variable);
            }
        }
    }
    (String::new(), DocKind::Unknown)
}
