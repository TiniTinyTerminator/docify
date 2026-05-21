use std::path::Path;
use super::{DocExtractor, DocItem, DocKind, DocLanguage, TagKind};
use super::common::{
    build_item, item_has_content,
    collect_c_block, collect_line_block, next_decl_sym,
    first_ident, func_name_before_paren,
};

pub struct CExtractor;
pub struct CppExtractor;

impl DocExtractor for CExtractor {
    fn extensions(&self) -> &[&str] { &["c", "h"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_c_style(src, path, &DocLanguage::C)
    }
}

impl DocExtractor for CppExtractor {
    fn extensions(&self) -> &[&str] {
        &["cpp", "cc", "cxx", "c++", "hpp", "hh", "hxx", "cu", "hip", "sycl", "ispc"]
    }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_c_style(src, path, &DocLanguage::Cpp)
    }
}

pub(super) fn extract_c_style(src: &str, file: &Path, lang: &DocLanguage) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    let mut brace_depth: usize = 0;
    let mut ns_stack: Vec<(usize, String)> = Vec::new();
    let mut pending_ns: Option<String> = None;

    // Doxygen group tracking: @defgroup/@addtogroup/@{/@}
    let mut group_stack:   Vec<String>   = Vec::new();
    let mut pending_group: Option<String> = None;

    while i < lines.len() {
        let t = lines[i].trim();

        if (t.starts_with("/**") && !t.starts_with("/***/")) || t.starts_with("/*!") {
            let (block, end) = collect_c_block(&lines, i);
            apply_group_directives(&block, &mut group_stack, &mut pending_group);
            if !is_pure_group_block(&block) {
                let sym = next_decl_sym(&lines, end + 1);
                if !is_c_conditional_directive(sym) {
                    let (name, kind) = detect_c_symbol(sym);
                    let ns  = ns_stack.last().map(|(_, p)| p.as_str()).unwrap_or("");
                    let mut item = build_item(block, qualify_name(&name, ns), kind, file, i + 1, lang.clone(), sym.to_string());
                    item.meta.group = ingroup_from_tags(&item) .or_else(|| group_stack.last().cloned());
                    if item_has_content(&item) { items.push(item); }
                }
            }
            i = end + 1;
            continue;
        }

        if t.starts_with("///") && !t.starts_with("////") {
            let (block, end) = collect_line_block(&lines, i, "///");
            apply_group_directives(&block, &mut group_stack, &mut pending_group);
            if !is_pure_group_block(&block) {
                let sym = next_decl_sym(&lines, end + 1);
                if !is_c_conditional_directive(sym) {
                    let (name, kind) = detect_c_symbol(sym);
                    let ns  = ns_stack.last().map(|(_, p)| p.as_str()).unwrap_or("");
                    let mut item = build_item(block, qualify_name(&name, ns), kind, file, i + 1, lang.clone(), sym.to_string());
                    item.meta.group = ingroup_from_tags(&item).or_else(|| group_stack.last().cloned());
                    if item_has_content(&item) { items.push(item); }
                }
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

// ── Doxygen group helpers ─────────────────────────────────────────────────────

/// Process any `@defgroup`, `@addtogroup`, `@{`, `@}` lines in `block` and
/// update `group_stack` / `pending_group` accordingly.
fn apply_group_directives(
    block: &[String],
    group_stack:   &mut Vec<String>,
    pending_group: &mut Option<String>,
) {
    for line in block {
        let t = line.trim();
        if t == "@{" || t == "{" {
            let g = pending_group.take().or_else(|| group_stack.last().cloned());
            if let Some(g) = g { group_stack.push(g); }
        } else if t == "@}" || t == "}" {
            group_stack.pop();
        } else if let Some(rest) = t.strip_prefix("@defgroup ")
                .or_else(|| t.strip_prefix("\\defgroup "))
                .or_else(|| t.strip_prefix("@addtogroup "))
                .or_else(|| t.strip_prefix("\\addtogroup "))
        {
            let name = first_ident(rest);
            if !name.is_empty() && pending_group.is_none() {
                *pending_group = Some(name);
            }
        }
    }
}

/// Return `true` when the block contains only group-control directives
/// (`@{`, `@}`, `@defgroup`, `@addtogroup`) and no real documentation.
fn is_pure_group_block(block: &[String]) -> bool {
    let non_empty: Vec<&str> = block.iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if non_empty.is_empty() { return false; }
    non_empty.iter().all(|l| {
        *l == "@{" || *l == "@}" || *l == "{" || *l == "}"
            || l.starts_with("@defgroup ")  || l.starts_with("\\defgroup ")
            || l.starts_with("@addtogroup ") || l.starts_with("\\addtogroup ")
            || *l == "@defgroup" || *l == "@addtogroup"
    })
}

/// Extract an explicit `@ingroup groupname` from an item's tag list.
fn ingroup_from_tags(item: &DocItem) -> Option<String> {
    item.tags.iter()
        .find(|t| t.kind == TagKind::Other("ingroup".to_string()))
        .map(|t| first_ident(&t.text))
        .filter(|s| !s.is_empty())
}
