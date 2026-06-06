/// clang-bridge-backed C/C++ extractor.
///
/// Enabled by the `clang-bridge` feature. When active, `extract_file` routes
/// all C/C++ files through this module, using the full clang AST (libclang-cpp)
/// for accurate extraction of doc comments, USRs, and signatures.
use std::path::Path;

use crate::extract::{DocItem, DocKind, DocLanguage, DocMeta, DocTag, TagKind};

/// Parse `path` via clang-bridge and return documented items.
pub fn extract_file_bridge(path: &Path, extra_args: &[&str]) -> Vec<DocItem> {
    let idx = clang_bridge::Index::new();
    let source = match path.to_str() {
        Some(s) => s,
        None => return vec![],
    };

    // Default to C++ for .h files (common in C++ projects); caller can override.
    let lang_flag = lang_flag_for(path);
    let mut args: Vec<&str> = vec![lang_flag];
    args.extend_from_slice(extra_args);

    let tu = match idx.parse(source, &args) {
        Some(tu) => tu,
        None => return vec![],
    };

    tu.doc_items()
        .map(|item| bridge_item_to_doc(item, path))
        .collect()
}

fn bridge_item_to_doc(item: clang_bridge::doc::DocItem, path: &Path) -> DocItem {
    let lang = lang_for(path);
    let kind = kind_for(&item.kind);

    // Split brief from body: brief is the first non-empty paragraph.
    let (brief, body) = split_brief_body(&item.full_comment, &item.brief);

    // Parse @param / @return / @note tags from the full comment.
    let tags = parse_tags(&item.full_comment);

    let file = if item.file.is_empty() {
        path.to_path_buf()
    } else {
        std::path::PathBuf::from(&item.file)
    };

    DocItem {
        name:      item.name,
        kind,
        brief,
        body,
        tags,
        file,
        line:      item.line as usize,
        lang,
        signature: item.signature,
        meta:      DocMeta {
            template_params: vec![],
            access:          None,
            parent:          None,
            attrs:           vec![],
            group:           None,
            package:         None,
        },
    }
}

fn lang_for(path: &Path) -> DocLanguage {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "c" | "h" => DocLanguage::C,
        _ => DocLanguage::Cpp,
    }
}

fn lang_flag_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "c" => "-xc",
        _ => "-xc++",
    }
}

fn kind_for(kind: &str) -> DocKind {
    match kind {
        "function" | "method" => DocKind::Function,
        "record" => DocKind::Class,
        "enum" => DocKind::Enum,
        "typedef" => DocKind::Typedef,
        "var" | "field" | "enumconst" => DocKind::Variable,
        _ => DocKind::Unknown,
    }
}

fn split_brief_body(full: &str, brief_hint: &str) -> (String, String) {
    if full.is_empty() {
        return (brief_hint.to_string(), String::new());
    }
    // Brief = text up to the first blank line or @tag.
    let mut brief_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;
    for line in full.lines() {
        let trimmed = line.trim();
        if !in_body && (trimmed.is_empty() || trimmed.starts_with('@') || trimmed.starts_with('\\')) {
            in_body = true;
        }
        if in_body {
            body_lines.push(line);
        } else {
            brief_lines.push(trimmed.to_string());
        }
    }
    let brief = if brief_lines.is_empty() {
        brief_hint.to_string()
    } else {
        brief_lines.join(" ")
    };
    (brief, body_lines.join("\n").trim().to_string())
}

fn parse_tags(comment: &str) -> Vec<DocTag> {
    let mut tags = Vec::new();
    let mut current_tag: Option<(TagKind, Option<String>, Vec<String>)> = None;

    let flush = |tags: &mut Vec<DocTag>, tag: Option<(TagKind, Option<String>, Vec<String>)>| {
        if let Some((kind, name, lines)) = tag {
            tags.push(DocTag {
                kind,
                name,
                text: lines.join(" ").trim().to_string(),
            });
        }
    };

    for line in comment.lines() {
        let trimmed = line.trim();
        // Match @param name desc, @return desc, @note desc, etc.
        if let Some(rest) = trimmed.strip_prefix('@').or_else(|| trimmed.strip_prefix('\\')) {
            flush(&mut tags, current_tag.take());
            let mut parts = rest.splitn(2, char::is_whitespace);
            let tag_word = parts.next().unwrap_or("");
            let rest2 = parts.next().unwrap_or("").trim();
            match tag_word {
                "param" | "param[in]" | "param[out]" | "param[in,out]" => {
                    let mut p = rest2.splitn(2, char::is_whitespace);
                    let name = p.next().unwrap_or("").to_string();
                    let desc = p.next().unwrap_or("").trim().to_string();
                    current_tag = Some((TagKind::Param, Some(name), vec![desc]));
                }
                "return" | "returns" => {
                    current_tag = Some((TagKind::Return, None, vec![rest2.to_string()]));
                }
                "note" => {
                    current_tag = Some((TagKind::Note, None, vec![rest2.to_string()]));
                }
                "warning" | "warn" => {
                    current_tag = Some((TagKind::Warning, None, vec![rest2.to_string()]));
                }
                "deprecated" => {
                    current_tag = Some((TagKind::Deprecated, None, vec![rest2.to_string()]));
                }
                "see" | "sa" => {
                    current_tag = Some((TagKind::See, None, vec![rest2.to_string()]));
                }
                other => {
                    current_tag = Some((TagKind::Other(other.to_string()), None, vec![rest2.to_string()]));
                }
            }
        } else if let Some((_, _, ref mut lines)) = current_tag {
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }
    flush(&mut tags, current_tag);
    tags
}
