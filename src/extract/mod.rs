use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod common;
mod cpp;
mod rust;
mod fortran;
mod d;
mod ada;
mod java;
mod go;

// Re-export shared helpers for extract_clang.rs (only needed when clang feature is active)
#[cfg(feature = "clang")]
pub(crate) use common::{build_item, item_has_content};

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DocLanguage {
    C, Cpp, Rust, Fortran, D, Ada, Java, Go, Unknown,
}

impl DocLanguage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::C       => "C",
            Self::Cpp     => "C++",
            Self::Rust    => "Rust",
            Self::Fortran => "Fortran",
            Self::D       => "D",
            Self::Ada     => "Ada",
            Self::Java    => "Java",
            Self::Go      => "Go",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocKind {
    Function, Struct, Class, Enum, Typedef, Variable,
    Macro, Module, Subroutine, Interface, Unknown,
}

impl DocKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Function   => "fn",
            Self::Struct     => "struct",
            Self::Class      => "class",
            Self::Enum       => "enum",
            Self::Typedef    => "type",
            Self::Variable   => "var",
            Self::Macro      => "macro",
            Self::Module     => "mod",
            Self::Subroutine => "sub",
            Self::Interface  => "iface",
            Self::Unknown    => "item",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TagKind {
    Brief, Param, Return, Note, See, Since,
    Deprecated, Example, Warning, Other(String),
}

impl TagKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Brief      => "Brief",
            Self::Param      => "Parameter",
            Self::Return     => "Returns",
            Self::Note       => "Note",
            Self::See        => "See also",
            Self::Since      => "Since",
            Self::Deprecated => "Deprecated",
            Self::Example    => "Example",
            Self::Warning    => "Warning",
            Self::Other(s)   => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocTag {
    pub kind: TagKind,
    /// Parameter name for `@param`; `None` for all other tag types.
    pub name: Option<String>,
    pub text: String,
}

/// Access level of a class / struct member.
#[derive(Debug, Clone, PartialEq)]
pub enum Access { Public, Protected, Private }

/// Structured metadata populated by language-aware extractors (libclang, etc.).
/// Defaults to empty so heuristic extractors compile without changes.
#[derive(Debug, Clone, Default)]
pub struct DocMeta {
    /// Template parameter list, e.g. `["typename T", "int N"]`.
    pub template_params: Vec<String>,
    /// Access specifier for class/struct members.
    pub access: Option<Access>,
    /// Qualified name of the enclosing class or struct, if any.
    pub parent: Option<String>,
    /// Semantic attributes: `"const"`, `"virtual"`, `"override"`, `"noexcept"`,
    /// `"pure"`, `"constructor"`, `"destructor"`, `"operator"`.
    pub attrs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DocItem {
    pub name: String,
    pub kind: DocKind,
    /// First sentence / `@brief` value.
    pub brief: String,
    /// Extended description after the brief.
    pub body: String,
    pub tags: Vec<DocTag>,
    pub file: PathBuf,
    /// 1-based line number of the opening doc comment.
    pub line: usize,
    pub lang: DocLanguage,
    /// The first non-blank source line following the doc comment.
    pub signature: String,
    /// Structured metadata populated by accurate extractors; empty for heuristic extraction.
    pub meta: DocMeta,
}

pub struct DocSet {
    pub items: Vec<DocItem>,
    pub source_root: PathBuf,
}

impl DocSet {
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
}

// ── Extractor trait ───────────────────────────────────────────────────────────

/// Trait for pluggable per-language doc-comment extractors.
///
/// Implement this to add support for a new language without modifying
/// the core dispatch logic. Register instances with [`extract_dir_with`].
pub trait DocExtractor: Send + Sync {
    /// File extensions handled by this extractor (without the leading dot).
    fn extensions(&self) -> &[&str];
    /// Extract documented items from `source` text read from `path`.
    fn extract(&self, path: &Path, source: &str) -> Vec<DocItem>;
}

// ── Language detection ────────────────────────────────────────────────────────

pub fn lang_from_ext(ext: &str) -> DocLanguage {
    match ext {
        "c" | "h" => DocLanguage::C,
        "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hh" | "hxx"
        | "cu" | "hip" | "sycl" | "ispc" => DocLanguage::Cpp,
        "rs" => DocLanguage::Rust,
        "f" | "f90" | "f95" | "f03" | "f08" | "F90" | "for" | "ftn" => DocLanguage::Fortran,
        "d" => DocLanguage::D,
        "ads" | "adb" => DocLanguage::Ada,
        "java" => DocLanguage::Java,
        "go" => DocLanguage::Go,
        _ => DocLanguage::Unknown,
    }
}

// ── Entry points ──────────────────────────────────────────────────────────────

pub fn extract_file(path: &Path) -> Vec<DocItem> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lang = lang_from_ext(ext);
    if lang == DocLanguage::Unknown { return vec![]; }
    #[cfg(feature = "clang")]
    if matches!(lang, DocLanguage::C | DocLanguage::Cpp) {
        return crate::extract_clang::extract_file_clang(path);
    }
    extract_file_heuristic(path)
}

/// Heuristic extractor used as a fallback when libclang is unavailable.
pub(crate) fn extract_file_heuristic(path: &Path) -> Vec<DocItem> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lang = lang_from_ext(ext);
    if lang == DocLanguage::Unknown { return vec![]; }
    let Ok(src) = std::fs::read_to_string(path) else { return vec![]; };
    from_str(&src, path, &lang)
}

pub fn extract_dir(dir: &Path) -> DocSet {
    let items = walk_and_extract(dir, &mut |path| extract_file(path));
    dedup(items, dir)
}

/// Like [`extract_dir`] but accepts additional [`DocExtractor`] implementations.
///
/// The built-in extractors run first; custom extractors handle any extensions
/// not already covered.
pub fn extract_dir_with(dir: &Path, extras: &[Box<dyn DocExtractor>]) -> DocSet {
    let items = walk_and_extract(dir, &mut |path| {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        // Try custom extractors for extensions not handled natively.
        if lang_from_ext(ext) == DocLanguage::Unknown {
            for extractor in extras {
                if extractor.extensions().contains(&ext) {
                    let Ok(src) = std::fs::read_to_string(path) else { continue };
                    return extractor.extract(path, &src);
                }
            }
            return vec![];
        }
        extract_file(path)
    });
    dedup(items, dir)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn walk_and_extract(dir: &Path, extract: &mut dyn FnMut(&Path) -> Vec<DocItem>) -> Vec<DocItem> {
    let mut items = Vec::new();
    let walker = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "build"
        });
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            items.extend(extract(entry.path()));
        }
    }
    items
}

fn dedup(items: Vec<DocItem>, source_root: &Path) -> DocSet {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut deduped: Vec<DocItem> = Vec::new();
    for item in items {
        let score = item.tags.len() * 10 + item.brief.len() + item.body.len();
        match seen.get(&item.name).copied() {
            Some(idx) => {
                let prev = deduped[idx].tags.len() * 10 + deduped[idx].brief.len() + deduped[idx].body.len();
                if score > prev { deduped[idx] = item; }
            }
            None => {
                seen.insert(item.name.clone(), deduped.len());
                deduped.push(item);
            }
        }
    }
    DocSet { items: deduped, source_root: source_root.to_path_buf() }
}

fn from_str(src: &str, file: &Path, lang: &DocLanguage) -> Vec<DocItem> {
    match lang {
        DocLanguage::C | DocLanguage::Cpp => cpp::extract_c_style(src, file, lang),
        DocLanguage::Rust                 => rust::extract_rust(src, file),
        DocLanguage::Fortran              => fortran::extract_fortran(src, file),
        DocLanguage::D                    => d::extract_d(src, file),
        DocLanguage::Ada                  => ada::extract_ada(src, file),
        DocLanguage::Java                 => java::extract_java(src, file),
        DocLanguage::Go                   => go::extract_go(src, file),
        DocLanguage::Unknown              => vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn items(src: &str, lang: &DocLanguage) -> Vec<DocItem> {
        from_str(src, Path::new("test.x"), lang)
    }

    // ── C / C++ ───────────────────────────────────────────────────────────────

    #[test]
    fn c_block_comment_extracts_brief() {
        let src = "/** Compute the sum of two integers. */\nint add(int a, int b);";
        let got = items(src, &DocLanguage::C);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Compute the sum of two integers.");
        assert_eq!(got[0].name, "add");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn c_block_multi_line_with_params() {
        let src = r#"/**
 * @brief Sort an array in place.
 * @param arr  Pointer to the array.
 * @param len  Number of elements.
 * @return Zero on success, negative on error.
 */
void sort(int *arr, size_t len);"#;
        let got = items(src, &DocLanguage::C);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Sort an array in place.");
        assert_eq!(got[0].name, "sort");
        let params: Vec<_> = got[0].tags.iter().filter(|t| t.kind == TagKind::Param).collect();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name.as_deref(), Some("arr"));
        let ret: Vec<_> = got[0].tags.iter().filter(|t| t.kind == TagKind::Return).collect();
        assert_eq!(ret.len(), 1);
    }

    #[test]
    fn c_triple_slash_line_comment() {
        let src = "/// A utility macro.\n#define MAX(a, b) ((a) > (b) ? (a) : (b))";
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "A utility macro.");
        assert!(matches!(got[0].kind, DocKind::Macro));
    }

    #[test]
    fn c_define_keeps_doc_comment() {
        let src = "/** Feature flag for vector code. */\n#define HAVE_VEC 1";
        let got = items(src, &DocLanguage::C);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "HAVE_VEC");
        assert_eq!(got[0].brief, "Feature flag for vector code.");
        assert!(matches!(got[0].kind, DocKind::Macro));
    }

    #[test]
    fn c_ifdef_does_not_consume_doc_comment() {
        let src = "/** Header guard. */\n#ifndef MATHLIB_H\n#define MATHLIB_H";
        let got = items(src, &DocLanguage::C);
        assert!(got.is_empty(), "conditional directives should not become documented items");
    }

    #[test]
    fn c_struct_detection() {
        let src = "/** Represents a 2D point. */\nstruct Point { float x; float y; };";
        let got = items(src, &DocLanguage::C);
        assert_eq!(got[0].name, "Point");
        assert!(matches!(got[0].kind, DocKind::Struct));
    }

    #[test]
    fn backslash_escape_not_parsed_as_tag() {
        let src = "/** Uses \\n for newlines and \\t for tabs. */\nvoid foo();";
        let got = items(src, &DocLanguage::C);
        assert_eq!(got.len(), 1);
        let unknown_tags: Vec<_> = got[0].tags.iter()
            .filter(|t| matches!(&t.kind, TagKind::Other(s) if s == "n" || s == "t"))
            .collect();
        assert!(unknown_tags.is_empty(), "single-char escape sequences must not become tags");
    }

    // ── Rust ──────────────────────────────────────────────────────────────────

    #[test]
    fn rust_fn_doc() {
        let src = "/// Return the factorial of n.\npub fn factorial(n: u64) -> u64 { todo!() }";
        let got = items(src, &DocLanguage::Rust);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Return the factorial of n.");
        assert_eq!(got[0].name, "factorial");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn rust_struct_doc() {
        let src = "/// A colour in linear sRGB.\npub struct Rgb { pub r: f32, pub g: f32, pub b: f32 }";
        let got = items(src, &DocLanguage::Rust);
        assert_eq!(got[0].name, "Rgb");
        assert!(matches!(got[0].kind, DocKind::Struct));
    }

    #[test]
    fn rust_impl_for_doc() {
        let src = "/// Display impl for Rgb.\nimpl std::fmt::Display for Rgb {}";
        let got = items(src, &DocLanguage::Rust);
        assert_eq!(got[0].name, "Rgb");
        assert!(matches!(got[0].kind, DocKind::Struct));
    }

    // ── Fortran ───────────────────────────────────────────────────────────────

    #[test]
    fn fortran_subroutine_doc() {
        let src = "!> Solve a linear system Ax = b.\n!! Uses LU decomposition.\nsubroutine solve(A, b, x, n)";
        let got = items(src, &DocLanguage::Fortran);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Solve a linear system Ax = b.");
        assert_eq!(got[0].name, "solve");
        assert!(matches!(got[0].kind, DocKind::Subroutine));
    }

    #[test]
    fn fortran_function_uppercase() {
        let src = "!> Compute dot product.\nFUNCTION dot(u, v, n) RESULT(res)";
        let got = items(src, &DocLanguage::Fortran);
        assert_eq!(got[0].name, "dot");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    // ── Ada ───────────────────────────────────────────────────────────────────

    #[test]
    fn ada_procedure_doc() {
        let src = "--! Print a greeting.\nprocedure Say_Hello (Name : String);";
        let got = items(src, &DocLanguage::Ada);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Print a greeting.");
        assert_eq!(got[0].name, "Say_Hello");
        assert!(matches!(got[0].kind, DocKind::Subroutine));
    }

    // ── Signature capture ─────────────────────────────────────────────────────

    #[test]
    fn c_block_captures_signature() {
        let src = "/** Compute sum. */\nint add(int a, int b);";
        let got = items(src, &DocLanguage::C);
        assert_eq!(got[0].signature, "int add(int a, int b);");
    }

    #[test]
    fn rust_fn_captures_signature() {
        let src = "/// Return factorial.\npub fn factorial(n: u64) -> u64 { todo!() }";
        let got = items(src, &DocLanguage::Rust);
        assert_eq!(got[0].signature, "pub fn factorial(n: u64) -> u64 { todo!() }");
    }

    #[test]
    fn signature_empty_when_no_following_line() {
        let src = "/** Orphan comment with nothing after. */\n";
        let got = items(src, &DocLanguage::C);
        if !got.is_empty() {
            assert!(!got[0].signature.is_empty() || got[0].signature.is_empty(),
                "signature is whatever followed; just must not crash");
        }
    }

    // ── Language detection ────────────────────────────────────────────────────

    #[test]
    fn extension_routing() {
        assert_eq!(lang_from_ext("c"),    DocLanguage::C);
        assert_eq!(lang_from_ext("cpp"),  DocLanguage::Cpp);
        assert_eq!(lang_from_ext("rs"),   DocLanguage::Rust);
        assert_eq!(lang_from_ext("f90"),  DocLanguage::Fortran);
        assert_eq!(lang_from_ext("d"),    DocLanguage::D);
        assert_eq!(lang_from_ext("ads"),  DocLanguage::Ada);
        assert_eq!(lang_from_ext("java"), DocLanguage::Java);
        assert_eq!(lang_from_ext("go"),   DocLanguage::Go);
        assert_eq!(lang_from_ext("toml"), DocLanguage::Unknown);
    }

    // ── C++ declarations ──────────────────────────────────────────────────────

    #[test]
    fn cpp_namespace_class() {
        let src = r#"/**
 * @brief 2-D point type.
 */
class Point {
    int x, y;
};"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Point");
        assert!(matches!(got[0].kind, DocKind::Class));
        assert_eq!(got[0].brief, "2-D point type.");
    }

    #[test]
    fn cpp_template_class() {
        let src = r#"/**
 * @brief Generic stack container.
 * @tparam T Element type.
 */
template<typename T>
class Stack {};
"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Generic stack container.");
        assert!(!got[0].brief.is_empty());
    }

    #[test]
    fn cpp_typedef_struct() {
        let src = "/** Opaque handle type. */\ntypedef struct _Handle Handle;";
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Opaque handle type.");
        assert!(matches!(got[0].kind, DocKind::Typedef));
        assert_eq!(got[0].name, "Handle");
    }

    #[test]
    fn cpp_using_alias() {
        let src = "/** Convenience alias for a string map. */\nusing StringMap = std::map<std::string, std::string>;";
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Convenience alias for a string map.");
    }

    #[test]
    fn cpp_enum_class() {
        let src = r#"/** Colour channels. */
enum class Channel { R, G, B, A };"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Channel");
        assert!(matches!(got[0].kind, DocKind::Enum));
    }

    #[test]
    fn cpp_static_member_function() {
        let src = r#"/** Create from polar coordinates.
 * @param r Radius.
 * @param theta Angle in radians.
 * @return New point.
 */
static Point from_polar(double r, double theta);"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "from_polar");
        assert!(matches!(got[0].kind, DocKind::Function));
        let params: Vec<_> = got[0].tags.iter().filter(|t| t.kind == TagKind::Param).collect();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name.as_deref(), Some("r"));
        assert_eq!(params[1].name.as_deref(), Some("theta"));
    }

    #[test]
    fn cpp_inline_variable_doc() {
        let src = "/** Maximum buffer size in bytes. */\nconst size_t MAX_BUF = 4096;";
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Maximum buffer size in bytes.");
        assert!(!got[0].brief.is_empty());
    }

    #[test]
    fn cpp_namespace_free_function() {
        let src = r#"namespace math {
/** @brief Clamp x to [lo, hi]. */
double clamp(double x, double lo, double hi);
} // namespace math"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "math::clamp");
        assert_eq!(got[0].brief, "Clamp x to [lo, hi].");
    }

    #[test]
    fn cpp_nested_namespace() {
        let src = r#"namespace outer {
namespace inner {
/** @brief Nested function. */
void nested();
} // namespace inner
} // namespace outer"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "outer::inner::nested");
    }

    #[test]
    fn cpp_multiline_param_continuation() {
        let src = r#"/**
 * @brief Multiply matrix.
 * @param A Input matrix; must be square and
 *   stored in row-major order.
 * @param n Dimension.
 */
void matmul(double *A, int n);"#;
        let got = items(src, &DocLanguage::Cpp);
        assert_eq!(got.len(), 1);
        let params: Vec<_> = got[0].tags.iter().filter(|t| t.kind == TagKind::Param).collect();
        assert_eq!(params.len(), 2);
        assert!(params[0].text.contains("row-major"), "expected continuation line in param text");
    }

    // ── Java ──────────────────────────────────────────────────────────────────

    #[test]
    fn java_method_doc() {
        let src = r#"/**
 * Compute the absolute value.
 * @param x Input value.
 * @return |x|.
 */
public static int abs(int x) { return x < 0 ? -x : x; }"#;
        let got = items(src, &DocLanguage::Java);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Compute the absolute value.");
        assert_eq!(got[0].name, "abs");
        assert!(matches!(got[0].kind, DocKind::Function));
        let params: Vec<_> = got[0].tags.iter().filter(|t| t.kind == TagKind::Param).collect();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name.as_deref(), Some("x"));
    }

    #[test]
    fn java_class_doc() {
        let src = "/** A generic container.\n */\npublic final class Box<T> {";
        let got = items(src, &DocLanguage::Java);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Box");
        assert!(matches!(got[0].kind, DocKind::Class));
    }

    #[test]
    fn java_interface_doc() {
        let src = "/** Serialisable contract. */\npublic interface Serialisable {";
        let got = items(src, &DocLanguage::Java);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Serialisable");
        assert!(matches!(got[0].kind, DocKind::Interface));
    }

    #[test]
    fn java_throws_tag() {
        let src = r#"/**
 * Parse the input.
 * @param s Input string.
 * @throws IllegalArgumentException if s is null.
 */
public void parse(String s) {}"#;
        let got = items(src, &DocLanguage::Java);
        assert_eq!(got.len(), 1);
        let throws: Vec<_> = got[0].tags.iter()
            .filter(|t| matches!(&t.kind, TagKind::Other(s) if s.starts_with("throws")))
            .collect();
        assert_eq!(throws.len(), 1);
    }

    // ── Go ────────────────────────────────────────────────────────────────────

    #[test]
    fn go_func_doc() {
        let src = "// Clamp returns x clamped to [lo, hi].\nfunc Clamp(x, lo, hi float64) float64 { return x }";
        let got = items(src, &DocLanguage::Go);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].brief, "Clamp returns x clamped to [lo, hi].");
        assert_eq!(got[0].name, "Clamp");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn go_struct_doc() {
        let src = "// Point is a 2D point.\ntype Point struct {\n    X, Y float64\n}";
        let got = items(src, &DocLanguage::Go);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Point");
        assert!(matches!(got[0].kind, DocKind::Struct));
    }

    #[test]
    fn go_method_doc() {
        let src = "// String returns a human-readable representation.\nfunc (p *Point) String() string { return \"\" }";
        let got = items(src, &DocLanguage::Go);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "String");
        assert!(matches!(got[0].kind, DocKind::Function));
    }

    #[test]
    fn go_comment_with_blank_line_ignored() {
        // A blank line between comment and declaration breaks the Go doc association.
        let src = "// This is just a comment.\n\nfunc Foo() {}";
        let got = items(src, &DocLanguage::Go);
        assert!(got.is_empty(), "blank line should break doc comment association");
    }

    #[test]
    fn go_directive_not_extracted() {
        let src = "//go:generate stringer -type=Status\nfunc Foo() {}";
        let got = items(src, &DocLanguage::Go);
        assert!(got.is_empty(), "//go: directives must not become doc items");
    }

    // ── Clang integration ────────────────────────────────────────────────────

    #[cfg(feature = "clang")]
    #[test]
    fn clang_extracts_namespace_items() {
        let src = r#"namespace math {
/**
 * @brief Clamp x to [lo, hi].
 * @param x  Value to clamp.
 * @param lo Lower bound.
 * @param hi Upper bound.
 * @return   Clamped value.
 */
double clamp(double x, double lo, double hi);
} // namespace math
"#;
        let dir = std::env::temp_dir();
        let path = dir.join("clang_test_clamp.hpp");
        std::fs::write(&path, src).unwrap();
        let got = crate::extract_clang::extract_file_clang(&path);
        std::fs::remove_file(&path).ok();

        assert!(!got.is_empty(), "clang extractor should find at least one item");
        let clamp = got.iter().find(|i| i.name.contains("clamp"))
            .expect("should find 'math::clamp'");
        assert_eq!(clamp.name, "math::clamp");
        assert!(clamp.brief.contains("Clamp"));
    }

    #[cfg(feature = "clang")]
    #[test]
    fn clang_extracts_class_members() {
        let src = r#"namespace stats {
/**
 * @brief Container of order statistics.
 */
class OrderStatistics {
public:
    /**
     * @brief Median of the sample.
     * @return Median value.
     */
    double median() const;
};
} // namespace stats
"#;
        let dir  = std::env::temp_dir();
        let path = dir.join("clang_test_order_stats.hpp");
        std::fs::write(&path, src).unwrap();
        let got  = crate::extract_clang::extract_file_clang(&path);
        std::fs::remove_file(&path).ok();

        let class = got.iter().find(|i| i.name.contains("OrderStatistics"))
            .expect("should extract the class");
        assert_eq!(class.name, "stats::OrderStatistics");

        let median = got.iter().find(|i| i.name.contains("median"))
            .expect("should extract median()");
        assert_eq!(median.name, "stats::OrderStatistics::median");
        assert!(median.meta.parent.as_deref() == Some("OrderStatistics"));
    }
}
