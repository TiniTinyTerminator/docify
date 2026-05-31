use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct RExtractor;

impl DocExtractor for RExtractor {
    fn extensions(&self) -> &[&str] { &["r", "R"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_r(src, path)
    }
}

fn extract_r(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        // Roxygen2: `#'` comment block
        if t.starts_with("#'") {
            let (block, end) = collect_roxygen(&lines, i);
            let sym = lines.get(end + 1).map(|l| l.trim()).unwrap_or("");
            let (name, kind) = detect_r_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::R, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn collect_roxygen(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("#'") {
            let content = t[2..].trim_start();
            // Roxygen tags: `@param x desc` → `@param x desc` (already compatible)
            out.push(content.to_string());
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn detect_r_symbol(line: &str) -> (String, DocKind) {
    // `name <- function(` or `name = function(`
    if let Some((lhs, rhs)) = line.split_once("<-").or_else(|| line.split_once('=')) {
        let name = lhs.trim().to_string();
        let rhs = rhs.trim();
        if name.is_empty() || name.contains(' ') { return (String::new(), DocKind::Unknown); }
        if rhs.starts_with("function") || rhs.starts_with("function ") {
            return (name, DocKind::Function);
        }
        if rhs.starts_with("setRefClass") || rhs.starts_with("R6Class") || rhs.starts_with("R5") {
            return (name, DocKind::Class);
        }
        return (name, DocKind::Variable);
    }
    let name = first_ident(line.trim());
    if !name.is_empty() { return (name, DocKind::Variable); }
    (String::new(), DocKind::Unknown)
}
