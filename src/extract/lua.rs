use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct LuaExtractor;

impl DocExtractor for LuaExtractor {
    fn extensions(&self) -> &[&str] { &["lua"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_lua(src, path)
    }
}

fn extract_lua(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        // LuaDoc: `---` starts a doc block; EmmyLua also uses `---@`
        if t.starts_with("---") {
            let (block, end) = collect_lua_doc(&lines, i);
            let sym = lines.get(end + 1).map(|l| l.trim()).unwrap_or("");
            let (name, kind) = detect_lua_symbol(sym);
            if !name.is_empty() {
                let item = build_item(block, name, kind, file, end + 2,
                    DocLanguage::Lua, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn collect_lua_doc(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("---") || t.starts_with("--") {
            let content = if t.starts_with("---") {
                t[3..].trim_start()
            } else {
                t[2..].trim_start()
            };
            // Convert EmmyLua `---@param x type desc` → `@param x desc`
            let content = if content.starts_with('@') {
                convert_emmylua_tag(content)
            } else {
                content.to_string()
            };
            out.push(content);
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn convert_emmylua_tag(s: &str) -> String {
    // `@param name type desc` → `@param name desc`
    if let Some(rest) = s.strip_prefix("@param ") {
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        match parts.as_slice() {
            [name, _ty, desc] => return format!("@param {name} {desc}"),
            [name, _ty] => return format!("@param {name}"),
            _ => {}
        }
    }
    // `@return type desc` → `@return desc`
    if let Some(rest) = s.strip_prefix("@return ") {
        let desc = rest.splitn(2, ' ').nth(1).unwrap_or(rest);
        return format!("@return {desc}");
    }
    s.to_string()
}

fn detect_lua_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();
    let t = t.strip_prefix("local ").unwrap_or(t);

    // `function name(` or `function module.name(`
    if let Some(r) = t.strip_prefix("function ") {
        let name = r.split('(').next().unwrap_or("").trim().to_string();
        return (name, DocKind::Function);
    }
    // `name = function(` or `name = function (` (local or global)
    if let Some((lhs, rhs)) = t.split_once('=') {
        let name = lhs.trim().to_string();
        let rhs = rhs.trim();
        if rhs.starts_with("function") && !name.is_empty() {
            return (name, DocKind::Function);
        }
        if !name.is_empty() {
            return (name, DocKind::Variable);
        }
    }
    // `M.name = {}` (module table)
    let name = first_ident(t);
    if !name.is_empty() && (t.contains('=') || t.contains('{')) {
        return (name, DocKind::Variable);
    }
    (String::new(), DocKind::Unknown)
}
