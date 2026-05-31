use super::common::{build_item, first_ident, item_has_content};
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use std::path::Path;

pub struct RubyExtractor;

impl DocExtractor for RubyExtractor {
    fn extensions(&self) -> &[&str] { &["rb"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_ruby(src, path)
    }
}

fn extract_ruby(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    // Track module/class scope for name qualification.
    let mut scope_stack: Vec<String> = Vec::new();

    while i < lines.len() {
        let t = lines[i].trim();

        // Track module/class scope (push on `module X` / `class X`, pop on `end`).
        if let Some(r) = t.strip_prefix("module ").or_else(|| t.strip_prefix("class ")) {
            let name = first_ident(r.split('<').next().unwrap_or(r));
            if !name.is_empty() { scope_stack.push(name); }
        } else if t == "end" {
            scope_stack.pop();
        }

        // YARD / RDoc: block of `#` lines starting with `##` or `# `.
        if is_ruby_doc_start(t) {
            let (block, end) = collect_ruby_doc(&lines, i);
            let sym = lines.get(end + 1).map(|l| l.trim()).unwrap_or("");
            let (name, kind) = detect_ruby_symbol(sym);
            if !name.is_empty() {
                let scope = scope_stack.last().map(String::as_str).unwrap_or("");
                let qualified = if !scope.is_empty() && kind != DocKind::Class
                    && kind != DocKind::Module
                {
                    format!("{scope}#{name}")
                } else { name };
                let item = build_item(block, qualified, kind, file, end + 2,
                    DocLanguage::Ruby, sym.to_string());
                if item_has_content(&item) { items.push(item); }
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    items
}

fn is_ruby_doc_start(t: &str) -> bool {
    t.starts_with("##") || (t.starts_with("# ") && !t.starts_with("# frozen_string_literal"))
        || t == "#"
}

fn collect_ruby_doc(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('#') {
            let content = t.strip_prefix("##").or_else(|| t.strip_prefix("# "))
                .or_else(|| t.strip_prefix('#'))
                .unwrap_or("").trim_start().to_string();
            out.push(content);
            i += 1;
        } else {
            break;
        }
    }
    (out, i.saturating_sub(1))
}

fn detect_ruby_symbol(line: &str) -> (String, DocKind) {
    let t = line.trim();
    if let Some(r) = t.strip_prefix("def ").or_else(|| t.strip_prefix("def self.")) {
        let name = r.split(|c| c == '(' || c == ' ' || c == '\n').next().unwrap_or("").trim().to_string();
        return (name, DocKind::Function);
    }
    if let Some(r) = t.strip_prefix("class ") {
        return (first_ident(r.split('<').next().unwrap_or(r)), DocKind::Class);
    }
    if let Some(r) = t.strip_prefix("module ") {
        return (first_ident(r), DocKind::Module);
    }
    if let Some(r) = t.strip_prefix("attr_accessor :").or_else(|| t.strip_prefix("attr_reader :"))
        .or_else(|| t.strip_prefix("attr_writer :"))
    {
        return (first_ident(r), DocKind::Variable);
    }
    (String::new(), DocKind::Unknown)
}
