use std::path::Path;
use super::{DocExtractor, DocItem, DocKind, DocLanguage};
use super::common::{build_item, item_has_content, collect_c_block, collect_line_block, next_non_blank, first_ident};

pub struct SwiftExtractor;

impl DocExtractor for SwiftExtractor {
    fn extensions(&self) -> &[&str] { &["swift"] }
    fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
        extract_swift(src, path)
    }
}

pub(super) fn extract_swift(src: &str, file: &Path) -> Vec<DocItem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let t = lines[i].trim();

        // `///` line-style doc comments
        if t.starts_with("///") && !t.starts_with("////") {
            let (block, end) = collect_line_block(&lines, i, "///");
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_swift_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::Swift, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        // `/** … */` block-style doc comments
        if t.starts_with("/**") && !t.starts_with("/***/") {
            let (block, end) = collect_c_block(&lines, i);
            let sym = next_non_blank(&lines, end + 1);
            let (name, kind) = detect_swift_symbol(sym);
            let item = build_item(block, name, kind, file, i + 1, DocLanguage::Swift, sym.to_string());
            if item_has_content(&item) { items.push(item); }
            i = end + 1;
            continue;
        }

        i += 1;
    }
    items
}

/// Access modifiers and declaration qualifiers that may precede the keyword.
const SWIFT_MODIFIERS: &[&str] = &[
    "public ",
    "private ",
    "internal ",
    "open ",
    "fileprivate ",
    "final ",
    "override ",
    "static ",
    "required ",
    "convenience ",
    "lazy ",
    "weak ",
    "unowned ",
    "mutating ",
    "nonmutating ",
    "dynamic ",
    "indirect ",
    "nonisolated ",
    "isolated ",
];

fn detect_swift_symbol(line: &str) -> (String, DocKind) {
    let mut t = line.trim();

    // Strip modifier keywords iteratively
    'outer: loop {
        // Skip attribute lines like `@objc`, `@discardableResult`, `@MainActor`
        if t.starts_with('@') {
            let after_at = &t[1..];
            let id_end = after_at.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(after_at.len());
            let after_id = after_at[id_end..].trim_start();
            if after_id.starts_with('(') {
                // Attribute with arguments: skip to closing paren
                let close = find_close_paren(after_id).unwrap_or(0);
                t = after_id[close + 1..].trim_start();
            } else {
                t = after_id;
            }
            continue 'outer;
        }
        for m in SWIFT_MODIFIERS {
            if let Some(rest) = t.strip_prefix(m) {
                t = rest.trim_start();
                continue 'outer;
            }
        }
        break;
    }

    // `class func` and `class var`/`let` are class-level members, not type declarations
    if let Some(r) = t.strip_prefix("class func ") { return (swift_func_name(r), DocKind::Function); }
    if let Some(r) = t.strip_prefix("class var ")  { return (first_ident(r),     DocKind::Variable); }
    if let Some(r) = t.strip_prefix("class let ")  { return (first_ident(r),     DocKind::Variable); }
    if let Some(r) = t.strip_prefix("func ")       { return (swift_func_name(r), DocKind::Function); }
    if let Some(r) = t.strip_prefix("init")        {
        // `init(`, `init?(`, `init!(`
        let name = if r.starts_with('(') || r.starts_with('?') || r.starts_with('!') {
            "init".to_string()
        } else {
            // Something like `initialize` — not `init`
            return (String::new(), DocKind::Unknown);
        };
        return (name, DocKind::Function);
    }
    if let Some(r) = t.strip_prefix("subscript")  { let _ = r; return ("subscript".to_string(), DocKind::Function); }
    if let Some(r) = t.strip_prefix("var ")        { return (first_ident(r), DocKind::Variable); }
    if let Some(r) = t.strip_prefix("let ")        { return (first_ident(r), DocKind::Variable); }
    if let Some(r) = t.strip_prefix("struct ")     { return (first_ident(r), DocKind::Struct); }
    if let Some(r) = t.strip_prefix("enum ")       { return (first_ident(r), DocKind::Enum); }
    if let Some(r) = t.strip_prefix("class ")      { return (first_ident(r), DocKind::Class); }
    if let Some(r) = t.strip_prefix("protocol ")   { return (first_ident(r), DocKind::Interface); }
    if let Some(r) = t.strip_prefix("typealias ")  { return (first_ident(r), DocKind::Typedef); }
    if let Some(r) = t.strip_prefix("extension ")  { return (first_ident(r), DocKind::Struct); }

    (String::new(), DocKind::Unknown)
}

/// Extract a Swift function name from the text after `func `.
/// Handles generic functions like `func greet<T>(` → `greet`.
fn swift_func_name(s: &str) -> String {
    // Operator functions may contain symbols like `==`, `+`, etc.
    // We split on `(` or `<` to isolate the name.
    s.split(|c: char| c == '(' || c == '<').next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Find the index of the closing `)` matching the first `(` in `s`.
fn find_close_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn items(src: &str) -> Vec<DocItem> {
        extract_swift(src, Path::new("test.swift"))
    }

    #[test]
    fn swift_func_line_doc() {
        let src = "/// Return the absolute value of x.\nfunc abs(_ x: Int) -> Int { return x < 0 ? -x : x }";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Return the absolute value of x.");
        assert_eq!(got[0].name, "abs");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn swift_func_block_doc() {
        let src = r#"/**
 * Compute the sum of two integers.
 * - Parameters:
 *   - a: First operand.
 *   - b: Second operand.
 * - Returns: a + b.
 */
func add(a: Int, b: Int) -> Int { return a + b }"#;
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Compute the sum of two integers.");
        assert_eq!(got[0].name, "add");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn swift_struct_doc() {
        let src = "/// A 2D point.\nstruct Point {\n    var x: Double\n    var y: Double\n}";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "A 2D point.");
        assert_eq!(got[0].name, "Point");
        assert!(matches!(got[0].kind, DocKind::Struct));
    }

    #[test]
    fn swift_class_doc() {
        let src = "/// Base view controller.\npublic class BaseViewController: UIViewController {";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "BaseViewController");
        assert!(matches!(got[0].kind, DocKind::Class));
    }

    #[test]
    fn swift_enum_doc() {
        let src = "/// Direction of travel.\nenum Direction { case north, south, east, west }";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Direction");
        assert!(matches!(got[0].kind, DocKind::Enum));
    }

    #[test]
    fn swift_protocol_doc() {
        let src = "/// Contract for drawable types.\nprotocol Drawable {\n    func draw()\n}";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Drawable");
        assert!(matches!(got[0].kind, DocKind::Interface));
    }

    #[test]
    fn swift_access_modifiers_stripped() {
        let src = "/// Public API surface.\npublic final override func compute() -> Int { return 42 }";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "compute");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn swift_no_doc_not_extracted() {
        let src = "// Plain comment\nfunc foo() {}";
        let got = items(src);
        assert!(got.is_empty(), "non-doc comments must not be extracted");
    }

    #[test]
    fn swift_typealias_doc() {
        let src = "/// Shorthand for a completion handler.\ntypealias CompletionHandler = (Result<Data, Error>) -> Void";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "CompletionHandler");
        assert!(matches!(got[0].kind, DocKind::Typedef));
    }

    #[test]
    fn swift_var_doc() {
        let src = "/// Maximum number of retries.\nlet maxRetries: Int = 3";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "maxRetries");
        assert!(matches!(got[0].kind, DocKind::Variable));
    }

    #[test]
    fn swift_init_doc() {
        let src = "/// Create a new point.\ninit(x: Double, y: Double) { self.x = x; self.y = y }";
        let got = items(src);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "init");
        assert!(matches!(got[0].kind, DocKind::Function));
    }
}
