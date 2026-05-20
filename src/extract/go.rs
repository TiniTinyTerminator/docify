use std::path::Path;
use super::{DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, first_ident};

pub(super) fn extract_go(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();

        // Go doc: consecutive `//` comment block, not a compiler directive
        if t.starts_with("//") && !t.starts_with("//go:") && !t.starts_with("//nolint") {
            let (block, end) = collect_go_comment(&lines, i);
            // Go doc convention: no blank line between comment and declaration
            let sym = lines.get(end + 1).map(|l| l.trim()).unwrap_or("");
            let (name, kind) = detect_go_symbol(sym);
            if kind != DocKind::Unknown {
                let item = build_item(block, name, kind, file, i + 1, DocLanguage::Go, sym.to_string());
                if item_has_content(&item) { items.push(item); }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    items
}

fn collect_go_comment(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("//") && !t.starts_with("//go:") && !t.starts_with("//nolint") {
            let content = t.strip_prefix("// ").unwrap_or_else(|| t.strip_prefix("//").unwrap_or(""));
            out.push(content.to_string());
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn detect_go_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();

    if let Some(rest) = t.strip_prefix("func ") {
        let rest = if rest.starts_with('(') {
            // Method with receiver — skip "(RecvType) "
            let close = rest.find(')').map(|j| j + 1).unwrap_or(0);
            rest[close..].trim_start()
        } else {
            rest
        };
        let name = rest.split('(').next().unwrap_or("").trim().to_string();
        if !name.is_empty() { return (name, DocKind::Function); }
    }

    if let Some(rest) = t.strip_prefix("type ") {
        let name = first_ident(rest);
        // Determine kind by inspecting what follows the name
        let after = rest[name.len()..].trim_start();
        if after.starts_with("struct") { return (name, DocKind::Struct); }
        if after.starts_with("interface") { return (name, DocKind::Interface); }
        if !name.is_empty() { return (name, DocKind::Typedef); }
    }

    if let Some(rest) = t.strip_prefix("var ")   { return (first_ident(rest), DocKind::Variable); }
    if let Some(rest) = t.strip_prefix("const ") { return (first_ident(rest), DocKind::Variable); }
    if let Some(rest) = t.strip_prefix("package ") { return (first_ident(rest), DocKind::Module); }

    (String::new(), DocKind::Unknown)
}
