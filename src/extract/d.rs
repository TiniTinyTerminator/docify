use std::path::Path;
use super::{DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, collect_c_block, collect_line_block, next_non_blank, first_ident};
use super::cpp::detect_c_symbol;

pub(super) fn extract_d(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();

        if t.starts_with("/++") {
            let (block, end) = collect_d_block(&lines, i);
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_d_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::D, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_d_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::D, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        if t.starts_with("///") && !t.starts_with("////") {
            let (block, end) = collect_line_block(&lines, i, "///");
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_d_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::D, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        i += 1;
    }
    items
}

fn collect_d_block(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let first = lines[start].trim();
    let after = first[3..].trim();

    if let Some(content) = after.strip_suffix("+/") {
        out.push(content.trim().to_string());
        return (out, start);
    }
    if !after.is_empty() { out.push(after.to_string()); }

    let mut i = start + 1;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.ends_with("+/") {
            let content = t.strip_suffix("+/").unwrap_or("").trim_start_matches('+').trim();
            if !content.is_empty() { out.push(content.to_string()); }
            return (out, i);
        }
        let content = t.strip_prefix("+ ").or_else(|| t.strip_prefix('+')).unwrap_or(t);
        out.push(content.to_string());
        i += 1;
    }
    (out, i.saturating_sub(1))
}

fn detect_d_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();
    let (name, kind) = detect_c_symbol(t);
    if kind != DocKind::Unknown { return (name, kind); }
    if let Some(r) = t.strip_prefix("interface ") { return (first_ident(r), DocKind::Interface); }
    if let Some(r) = t.strip_prefix("module ")    { return (first_ident(r), DocKind::Module); }
    (name, kind)
}
