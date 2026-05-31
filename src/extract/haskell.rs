use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct HaskellExtractor;

impl DocExtractor for HaskellExtractor {
    fn extensions(&self) -> &[&str] { &["hs", "lhs"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_haskell(src, path)
    }
}

fn extract_haskell(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    // Pre-scan for module name.
    let module = lines.iter()
        .find_map(|l| l.trim().strip_prefix("module ").map(|r| first_ident(r.split_whitespace().next().unwrap_or(r))))
        .unwrap_or_default();

    while i < lines.len() {
        let t = lines[i].trim();

        // Haddock: `-- |` starts a doc block (or `{- |` for multi-line).
        if t.starts_with("-- |") || t.starts_with("{- |") {
            let (block, end) = if t.starts_with("{- |") {
                collect_haddock_block(&lines, i)
            } else {
                collect_haddock_line(&lines, i)
            };
            let sym = lines.get(end + 1).map(|l| l.trim()).unwrap_or("");
            let (name, kind) = detect_haskell_symbol(sym, &module);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::Haskell, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn collect_haddock_line(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("-- |") {
            out.push(t[4..].trim().to_string());
            i += 1;
        } else if t.starts_with("--") && !out.is_empty() {
            // Continuation line (plain `--`)
            out.push(t[2..].trim().to_string());
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn collect_haddock_block(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let t = lines[start].trim();
    // First line: `{- | text`
    out.push(t[4..].trim().to_string());
    let mut i = start + 1;
    while i < lines.len() {
        let l = lines[i].trim();
        if let Some(before) = l.find("-}").map(|p| &l[..p]) {
            if !before.trim().is_empty() { out.push(before.trim().to_string()); }
            return (out, i);
        }
        out.push(l.to_string());
        i += 1;
    }
    (out, i.saturating_sub(1))
}

fn detect_haskell_symbol(line: &str, module: &str) -> (String, DocKind) {
    let t = line.trim();
    if t.is_empty() { return (String::new(), DocKind::Unknown); }

    // `data Name` / `newtype Name` / `type Name`
    if let Some(r) = t.strip_prefix("data ").or_else(|| t.strip_prefix("newtype ")) {
        let name = qualify(first_ident(r), module);
        return (name, DocKind::Struct);
    }
    if let Some(r) = t.strip_prefix("type ") {
        let name = qualify(first_ident(r), module);
        return (name, DocKind::Typedef);
    }
    if let Some(r) = t.strip_prefix("class ") {
        let name = qualify(first_ident(r), module);
        return (name, DocKind::Interface);
    }
    if let Some(r) = t.strip_prefix("module ") {
        return (first_ident(r), DocKind::Module);
    }

    // Function type signature: `name :: Type`  or `name, name2 :: Type`
    if let Some((lhs, _)) = t.split_once("::") {
        let name = first_ident(lhs.split(',').next().unwrap_or(lhs).trim());
        if !name.is_empty() && name.chars().next().map(|c| c.is_lowercase() || c == '_').unwrap_or(false) {
            return (qualify(name, module), DocKind::Function);
        }
    }

    // Function definition: `name args = ...`
    if let Some((lhs, _)) = t.split_once('=') {
        let name = first_ident(lhs.trim());
        if !name.is_empty() && name.chars().next().map(|c| c.is_lowercase() || c == '_').unwrap_or(false) {
            return (qualify(name, module), DocKind::Function);
        }
    }

    (String::new(), DocKind::Unknown)
}

fn qualify(name: String, module: &str) -> String {
    if name.is_empty() || module.is_empty() { name }
    else { format!("{module}.{name}") }
}
