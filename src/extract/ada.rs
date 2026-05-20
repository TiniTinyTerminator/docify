use std::path::Path;
use super::{DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, next_non_blank, ci_ident_after};

pub(super) fn extract_ada(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("--!") || t.starts_with("---") {
            let (block, end) = collect_ada_block(&lines, i);
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_ada_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::Ada, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn collect_ada_block(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("--!") || t.starts_with("---") {
            out.push(t[3..].trim_start().to_string());
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn detect_ada_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();
    let up = t.to_ascii_uppercase();
    if up.starts_with("PROCEDURE ") { return (ci_ident_after(t, "procedure "), DocKind::Subroutine); }
    if up.starts_with("FUNCTION ")  { return (ci_ident_after(t, "function "),  DocKind::Function); }
    if up.starts_with("PACKAGE ")   { return (ci_ident_after(t, "package "),   DocKind::Module); }
    if up.starts_with("TYPE ")      { return (ci_ident_after(t, "type "),       DocKind::Typedef); }
    (String::new(), DocKind::Unknown)
}
