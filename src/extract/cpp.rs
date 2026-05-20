use std::path::Path;
use super::{DocItem, DocKind, DocLanguage};
use super::common::{
    build_item, item_has_content,
    collect_c_block, collect_line_block, next_decl_sym,
    first_ident, func_name_before_paren,
};

pub(super) fn extract_c_style(src: &str, file: &Path, lang: &DocLanguage) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    let mut brace_depth: usize = 0;
    let mut ns_stack: Vec<(usize, String)> = Vec::new();
    let mut pending_ns: Option<String> = None;

    while i < lines.len() {
        let t = lines[i].trim();

        if (t.starts_with("/**") && !t.starts_with("/***/")) || t.starts_with("/*!") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_decl_sym(&lines, end + 1);
            if !is_c_conditional_directive(sym) {
                let (name, kind) = detect_c_symbol(sym);
                let ns  = ns_stack.last().map(|(_, p)| p.as_str()).unwrap_or("");
                let item = build_item(block, qualify_name(&name, ns), kind, file, i + 1, lang.clone(), sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }

        if t.starts_with("///") && !t.starts_with("////") {
            let (block, end) = collect_line_block(&lines, i, "///");
            let sym = next_decl_sym(&lines, end + 1);
            if !is_c_conditional_directive(sym) {
                let (name, kind) = detect_c_symbol(sym);
                let ns  = ns_stack.last().map(|(_, p)| p.as_str()).unwrap_or("");
                let item = build_item(block, qualify_name(&name, ns), kind, file, i + 1, lang.clone(), sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }

        if !t.starts_with("//") && !t.starts_with("/*") && !t.starts_with('*') {
            if let Some(rest) = t.strip_prefix("namespace") {
                let rest_ok = rest.is_empty()
                    || rest.starts_with(|c: char| c.is_whitespace() || c == '{');
                if rest_ok {
                    let name = first_ident(rest.trim_start());
                    if !name.is_empty() {
                        let path = match ns_stack.last() {
                            Some((_, p)) => format!("{p}::{name}"),
                            None         => name,
                        };
                        if t.contains('{') {
                            let opens  = t.chars().filter(|&c| c == '{').count();
                            let closes = t.chars().filter(|&c| c == '}').count();
                            brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
                            ns_stack.push((brace_depth, path));
                        } else {
                            pending_ns = Some(path);
                        }
                        i += 1;
                        continue;
                    }
                }
            }

            if pending_ns.is_some() && t.contains('{') {
                let path   = pending_ns.take().unwrap();
                let opens  = t.chars().filter(|&c| c == '{').count();
                let closes = t.chars().filter(|&c| c == '}').count();
                brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
                ns_stack.push((brace_depth, path));
                while ns_stack.last().map_or(false, |&(d, _)| brace_depth < d) {
                    ns_stack.pop();
                }
                i += 1;
                continue;
            }

            let opens  = t.chars().filter(|&c| c == '{').count();
            let closes = t.chars().filter(|&c| c == '}').count();
            brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
            while ns_stack.last().map_or(false, |&(d, _)| brace_depth < d) {
                ns_stack.pop();
            }
        }

        i += 1;
    }
    items
}

pub(crate) fn detect_c_symbol(line: &str) -> (String, DocKind) {
    let t = strip_c_qualifiers(line.trim());

    if let Some(r) = t.strip_prefix("struct ")        { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("class ")         { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("enum class ")    { return (first_ident(r), DocKind::Enum); }
    if let Some(r) = t.strip_prefix("enum ")          { return (first_ident(r), DocKind::Enum); }
    if let Some(r) = t.strip_prefix("namespace ")     { return (first_ident(r), DocKind::Module); }
    if let Some(r) = t.strip_prefix("#define ")       { return (first_ident(r), DocKind::Macro); }
    if t.starts_with("typedef ") {
        let candidate = t.trim_end_matches(';')
            .trim_end()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty())
            .last()
            .unwrap_or("")
            .to_string();
        if !candidate.is_empty()
            && !matches!(candidate.as_str(), "struct" | "union" | "enum" | "class")
        {
            return (candidate, DocKind::Typedef);
        }
        return (String::new(), DocKind::Typedef);
    }
    if t.starts_with("template") {
        return (String::new(), DocKind::Unknown);
    }
    if let Some(name) = func_name_before_paren(t) {
        return (name, DocKind::Function);
    }
    (String::new(), DocKind::Unknown)
}

fn qualify_name(name: &str, ns: &str) -> String {
    if name.is_empty() || ns.is_empty() { name.to_string() } else { format!("{ns}::{name}") }
}

fn is_c_conditional_directive(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("#if")
        || t.starts_with("#elif")
        || t.starts_with("#else")
        || t.starts_with("#endif")
}

fn strip_c_qualifiers(mut t: &str) -> &str {
    const QUALS: &[&str] = &[
        "static ", "inline ", "extern ", "explicit ", "virtual ",
        "constexpr ", "consteval ", "constinit ",
        "__inline ", "__inline__ ", "__forceinline ",
        "[[nodiscard]] ", "[[maybe_unused]] ",
    ];
    'outer: loop {
        for q in QUALS {
            if let Some(rest) = t.strip_prefix(q) {
                t = rest.trim_start();
                continue 'outer;
            }
        }
        break;
    }
    t
}
