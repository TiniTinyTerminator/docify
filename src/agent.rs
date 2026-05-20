use std::path::Path;

use serde::Serialize;

use crate::extract::{extract_dir, DocItem, DocLanguage, TagKind};

// ── JSON output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SymbolJson {
    pub name:      String,
    pub kind:      &'static str,
    pub lang:      &'static str,
    pub file:      String,
    pub line:      usize,
    pub brief:     String,
    pub signature: String,
    pub params:    Vec<ParamJson>,
    pub returns:   String,
    pub throws:    Vec<String>,
    pub notes:     Vec<String>,
    pub body:      String,
}

#[derive(Serialize)]
pub struct ParamJson {
    pub name: Option<String>,
    pub desc: String,
}

#[derive(Serialize)]
pub struct OutlineItem {
    pub name:  String,
    pub kind:  &'static str,
    pub lang:  &'static str,
    pub file:  String,
    pub line:  usize,
    pub brief: String,
}

#[derive(Serialize)]
pub struct ContextJson {
    pub doc:    SymbolJson,
    pub source: String,
}

// ── Conversions ───────────────────────────────────────────────────────────────

impl From<DocItem> for SymbolJson {
    fn from(item: DocItem) -> Self {
        let params: Vec<ParamJson> = item.tags.iter()
            .filter(|t| t.kind == TagKind::Param)
            .map(|t| ParamJson { name: t.name.clone(), desc: t.text.clone() })
            .collect();
        let returns = item.tags.iter()
            .find(|t| t.kind == TagKind::Return)
            .map(|t| t.text.clone())
            .unwrap_or_default();
        let throws: Vec<String> = item.tags.iter()
            .filter(|t| matches!(&t.kind, TagKind::Other(s) if s.starts_with("throws")))
            .map(|t| t.kind.label().to_string()
                + if t.text.is_empty() { "" } else { ": " }
                + &t.text)
            .collect();
        let notes: Vec<String> = item.tags.iter()
            .filter(|t| t.kind == TagKind::Note)
            .map(|t| t.text.clone())
            .collect();

        SymbolJson {
            file:      item.file.display().to_string(),
            kind:      item.kind.label(),
            lang:      item.lang.label(),
            name:      item.name,
            line:      item.line,
            brief:     item.brief,
            signature: item.signature,
            params,
            returns,
            throws,
            notes,
            body:      item.body,
        }
    }
}

impl From<&DocItem> for OutlineItem {
    fn from(item: &DocItem) -> Self {
        OutlineItem {
            name:  item.name.clone(),
            kind:  item.kind.label(),
            lang:  item.lang.label(),
            file:  item.file.display().to_string(),
            line:  item.line,
            brief: item.brief.clone(),
        }
    }
}

// ── Symbol lookup ─────────────────────────────────────────────────────────────

/// Find the best-matching documented symbol for `name` across `dirs`.
///
/// Match priority:
/// 1. Exact qualified name (`stats::mean`)
/// 2. Unqualified suffix (`mean` matches `stats::mean`)
/// 3. Case-insensitive substring on name
pub fn find_symbol(name: &str, dirs: &[&Path]) -> Option<DocItem> {
    let items = collect(dirs);

    // 1. Exact
    if let Some(item) = items.iter().find(|i| i.name == name) {
        return Some(item.clone());
    }
    // 2. Unqualified suffix
    let suffix = format!("::{name}");
    if let Some(item) = items.iter().find(|i| i.name.ends_with(&suffix)) {
        return Some(item.clone());
    }
    // 3. Case-insensitive substring
    let lower = name.to_ascii_lowercase();
    items.into_iter().find(|i| i.name.to_ascii_lowercase().contains(&lower))
}

/// Return all items whose name or brief contains `query` (case-insensitive).
pub fn search_symbols(query: &str, dirs: &[&Path]) -> Vec<DocItem> {
    let lower = query.to_ascii_lowercase();
    collect(dirs).into_iter()
        .filter(|i| {
            i.name.to_ascii_lowercase().contains(&lower)
                || i.brief.to_ascii_lowercase().contains(&lower)
        })
        .collect()
}

/// Return all documented symbols across `dirs`.
pub fn all_symbols(dirs: &[&Path]) -> Vec<DocItem> {
    collect(dirs)
}

// ── Source extraction ─────────────────────────────────────────────────────────

/// Extract the raw source text for `item` — doc comment + declaration + body.
///
/// Uses brace-depth tracking for C-family, Rust, Java, and Go.
/// Falls back to an `END keyword` scan for Fortran/Ada, then to a line window.
pub fn extract_source(item: &DocItem, max_lines: usize) -> String {
    let Ok(text) = std::fs::read_to_string(&item.file) else { return String::new() };
    let lines: Vec<&str> = text.lines().collect();
    let start = item.line.saturating_sub(1);
    if start >= lines.len() { return String::new(); }

    let end = match item.lang {
        DocLanguage::Fortran => find_fortran_end(&lines, start, &item.name, max_lines),
        DocLanguage::Ada     => find_ada_end(&lines, start, &item.name, max_lines),
        _                    => find_brace_end(&lines, start, max_lines),
    };

    lines[start..=end].join("\n")
}

// ── Block-end finders ─────────────────────────────────────────────────────────

fn find_brace_end(lines: &[&str], start: usize, max: usize) -> usize {
    let bound = (start + max).min(lines.len().saturating_sub(1));
    let mut depth: i32 = 0;
    let mut seen_open = false;

    for i in start..=bound {
        let line = lines[i];
        let mut in_str = false;
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' => in_str = !in_str,
                '{' if !in_str => { depth += 1; seen_open = true; }
                '}' if !in_str => { depth -= 1; }
                '/' if !in_str => {
                    if chars.peek() == Some(&'/') { break; } // line comment
                }
                _ => {}
            }
        }
        if seen_open && depth <= 0 {
            return i;
        }
    }
    // No opening brace — likely a forward declaration; return just the signature line
    if !seen_open {
        // Find the declaration line (first non-comment line after start)
        for i in start..=bound {
            let t = lines[i].trim();
            if !t.is_empty() && !t.starts_with("//") && !t.starts_with('*')
                && !t.starts_with("/**") && !t.starts_with("/*!") && !t.starts_with("///")
            {
                return i;
            }
        }
    }
    bound
}

fn find_fortran_end(lines: &[&str], start: usize, name: &str, max: usize) -> usize {
    let bound = (start + max).min(lines.len().saturating_sub(1));
    let bare = name.rsplit("::").next().unwrap_or(name).to_ascii_uppercase();
    for i in (start + 1)..=bound {
        let up = lines[i].trim().to_ascii_uppercase();
        if up.starts_with("END SUBROUTINE")
            || up.starts_with("END FUNCTION")
            || up.starts_with("END MODULE")
            || up.starts_with("END TYPE")
        {
            // Match on name suffix if present
            let after = up.split_whitespace().nth(2).unwrap_or("");
            if after.is_empty() || after == bare { return i; }
        }
    }
    bound
}

fn find_ada_end(lines: &[&str], start: usize, name: &str, max: usize) -> usize {
    let bound = (start + max).min(lines.len().saturating_sub(1));
    let bare = name.rsplit('.').next().unwrap_or(name).to_ascii_uppercase();
    for i in (start + 1)..=bound {
        let up = lines[i].trim().to_ascii_uppercase();
        if up.starts_with("END ") || up == "END;" {
            let after = up.split_whitespace().nth(1).unwrap_or("").trim_end_matches(';');
            if after.is_empty() || after == bare { return i; }
        }
    }
    bound
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn collect(dirs: &[&Path]) -> Vec<DocItem> {
    let mut all = Vec::new();
    for dir in dirs {
        all.extend(extract_dir(dir).items);
    }
    all
}
