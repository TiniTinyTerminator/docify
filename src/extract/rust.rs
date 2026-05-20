use std::path::Path;
use super::{DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, collect_c_block, collect_line_block, next_non_blank};

pub(super) fn extract_rust(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();

        if t.starts_with("///") && !t.starts_with("////") {
            let (block, end) = collect_line_block(&lines, i, "///");
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_rust_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::Rust, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_rust_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::Rust, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        i += 1;
    }
    items
}

fn detect_rust_symbol(line: &str) -> (String, DocKind) {
    let words: Vec<&str> = line.split_whitespace().collect();
    let skip = words.iter().take_while(|&&w| {
        matches!(w, "pub" | "async" | "unsafe" | "extern" | "default")
        || w.starts_with("pub(")
        || w.starts_with('"')
    }).count();

    let rest = &words[skip..];
    if rest.is_empty() { return (String::new(), DocKind::Unknown); }

    let keyword = rest[0];
    let name_raw = rest.get(1).copied().unwrap_or("")
        .split(['<', '(', '{', ':']).next().unwrap_or("");
    let name = name_raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string();

    match keyword {
        "fn"     => (name, DocKind::Function),
        "struct" => (name, DocKind::Struct),
        "enum"   => (name, DocKind::Enum),
        "trait"  => (name, DocKind::Interface),
        "type"   => (name, DocKind::Typedef),
        "mod"    => (name, DocKind::Module),
        "const"  => (name, DocKind::Variable),
        "static" => (name, DocKind::Variable),
        "impl"   => {
            if let Some(pos) = rest.iter().position(|w| *w == "for") {
                let after = rest.get(pos + 1).copied().unwrap_or("");
                let n = after.split('<').next().unwrap_or("").to_string();
                return (n, DocKind::Struct);
            }
            (name, DocKind::Struct)
        }
        _ => (String::new(), DocKind::Unknown),
    }
}
