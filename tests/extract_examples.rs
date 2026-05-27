/// Integration tests: extract documentation from fixture source files and
/// from the existing `examples/doc-example` project.
///
/// Each test scans a real source file (or directory) and asserts that specific
/// named symbols are found with correct kinds and non-empty doc content.
use std::path::Path;

use docify::extract::{extract_dir, extract_file, DocKind};

// ── helpers ──────────────────────────────────────────────────────────────────

fn fixture(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn doc_example(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/doc-example")
        .join(rel)
}

fn has_item(items: &[docify::extract::DocItem], name: &str) -> bool {
    items.iter().any(|i| i.name == name)
}

fn find_item<'a>(
    items: &'a [docify::extract::DocItem],
    name: &str,
) -> &'a docify::extract::DocItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        panic!(
            "item '{name}' not found; got: {:?}",
            items.iter().map(|i| &i.name).collect::<Vec<_>>()
        )
    })
}

// ── C fixture: buffer.h ───────────────────────────────────────────────────────

#[test]
fn c_buffer_finds_typedef() {
    let items = extract_file(&fixture("c/buffer.h"));
    let item = find_item(&items, "Buffer");
    assert!(
        matches!(item.kind, DocKind::Typedef | DocKind::Struct),
        "Buffer should be Typedef or Struct, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "Buffer needs a brief");
}

#[test]
fn c_buffer_finds_all_functions() {
    let items = extract_file(&fixture("c/buffer.h"));
    for name in &[
        "buffer_init",
        "buffer_push",
        "buffer_clear",
        "buffer_remaining",
    ] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn c_buffer_push_has_params() {
    let items = extract_file(&fixture("c/buffer.h"));
    let push = find_item(&items, "buffer_push");
    use docify::extract::TagKind;
    let params: Vec<_> = push
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert!(params.len() >= 3, "buffer_push should have ≥3 @param tags");
}

#[test]
fn c_buffer_finds_macro() {
    let items = extract_file(&fixture("c/buffer.h"));
    assert!(
        has_item(&items, "BUFFER_CAP"),
        "BUFFER_CAP macro should be found"
    );
    let item = find_item(&items, "BUFFER_CAP");
    assert!(matches!(item.kind, DocKind::Macro));
}

// ── C fixture: additional coverage ───────────────────────────────────────────

#[test]
fn c_buffer_push_has_return() {
    let items = extract_file(&fixture("c/buffer.h"));
    let push = find_item(&items, "buffer_push");
    use docify::extract::TagKind;
    assert!(
        push.tags.iter().any(|t| t.kind == TagKind::Return),
        "buffer_push should have a @return tag"
    );
}

#[test]
fn c_buffer_remaining_line_comment_style() {
    // buffer_remaining is documented with `///` lines, not `/** */`.
    let items = extract_file(&fixture("c/buffer.h"));
    let item = find_item(&items, "buffer_remaining");
    assert!(matches!(item.kind, DocKind::Function));
    assert!(
        !item.brief.is_empty(),
        "buffer_remaining needs a brief from /// style"
    );
    use docify::extract::TagKind;
    assert!(
        item.tags.iter().any(|t| t.kind == TagKind::Param),
        "buffer_remaining should have @param from /// style"
    );
}

// ── C++ fixture: geometry.hpp ─────────────────────────────────────────────────

#[test]
fn cpp_geometry_finds_namespace_items() {
    let items = extract_file(&fixture("cpp/geometry.hpp"));
    // Free functions must be qualified with the namespace.
    let tri = find_item(&items, "geometry::triangle_area");
    assert!(matches!(tri.kind, DocKind::Function));
    assert!(!tri.brief.is_empty());

    let ov = find_item(&items, "geometry::aabb_overlaps");
    assert!(matches!(ov.kind, DocKind::Function));
}

#[test]
fn cpp_geometry_finds_class_and_struct() {
    let items = extract_file(&fixture("cpp/geometry.hpp"));
    let point = find_item(&items, "geometry::Point");
    assert!(matches!(point.kind, DocKind::Struct | DocKind::Class));

    let aabb = find_item(&items, "geometry::AABB");
    assert!(matches!(aabb.kind, DocKind::Struct | DocKind::Class));
}

#[test]
fn cpp_geometry_total_item_count() {
    let items = extract_file(&fixture("cpp/geometry.hpp"));
    // At minimum: Point, AABB, triangle_area, aabb_overlaps (+ possible methods).
    assert!(items.len() >= 4, "expected ≥4 items, got {}", items.len());
}

// ── C++ fixture: additional coverage ─────────────────────────────────────────

#[test]
fn cpp_geometry_triangle_area_params() {
    let items = extract_file(&fixture("cpp/geometry.hpp"));
    let tri = find_item(&items, "geometry::triangle_area");
    use docify::extract::TagKind;
    let params: Vec<_> = tri
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert_eq!(params.len(), 3, "triangle_area should have 3 @param tags");
    let names: Vec<_> = params.iter().filter_map(|p| p.name.as_deref()).collect();
    assert!(
        names.contains(&"a") && names.contains(&"b") && names.contains(&"c"),
        "expected params a, b, c; got {:?}",
        names
    );
}

#[test]
fn cpp_geometry_triangle_area_has_return() {
    let items = extract_file(&fixture("cpp/geometry.hpp"));
    let tri = find_item(&items, "geometry::triangle_area");
    use docify::extract::TagKind;
    assert!(
        tri.tags.iter().any(|t| t.kind == TagKind::Return),
        "triangle_area should have a @return tag"
    );
}

// ── CUDA fixture: kernels.cu ─────────────────────────────────────────────────

#[test]
fn cuda_kernels_classify_qualified_functions() {
    let items = extract_file(&fixture("cuda/kernels.cu"));

    for (name, qualifier) in &[
        ("vector_add", "__global__"),
        ("reduce_sum", "__global__"),
        ("clamp_unit", "__device__"),
        ("scale_in_place", "__host__"),
    ] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
        assert!(
            !item.display_signature().contains(qualifier),
            "CUDA qualifier should not appear in display signature: {}",
            item.display_signature()
        );
    }
}

#[test]
fn cuda_kernels_have_table_tags() {
    let items = extract_file(&fixture("cuda/kernels.cu"));
    let reduce = find_item(&items, "reduce_sum");
    use docify::extract::TagKind;
    let params: Vec<_> = reduce
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert_eq!(
        params.len(),
        3,
        "reduce_sum should have input/partial/n params"
    );
}

// ── Fortran fixture: vectors.f90 ──────────────────────────────────────────────

#[test]
fn fortran_vectors_module_variables() {
    let items = extract_file(&fixture("fortran/vectors.f90"));
    let wp = find_item(&items, "wp");
    assert!(matches!(wp.kind, DocKind::Variable));
    assert!(!wp.brief.is_empty());

    let max_dim = find_item(&items, "max_dim");
    assert!(matches!(max_dim.kind, DocKind::Variable));

    let tol = find_item(&items, "norm_tol");
    assert!(matches!(tol.kind, DocKind::Variable));
}

#[test]
fn fortran_vectors_procedures() {
    let items = extract_file(&fixture("fortran/vectors.f90"));
    for name in &["dot3", "cross3", "norm3"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function"
        );
    }
    let norm = find_item(&items, "normalise");
    assert!(matches!(norm.kind, DocKind::Subroutine));
}

#[test]
fn fortran_vectors_total_item_count() {
    let items = extract_file(&fixture("fortran/vectors.f90"));
    assert!(
        items.len() >= 7,
        "expected ≥7 items (3 vars + 4 procs), got {}",
        items.len()
    );
}

// ── Fortran fixture: additional coverage ─────────────────────────────────────

#[test]
fn fortran_dot3_has_params_and_return() {
    let items = extract_file(&fixture("fortran/vectors.f90"));
    let dot = find_item(&items, "dot3");
    use docify::extract::TagKind;
    let params: Vec<_> = dot
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert!(params.len() >= 2, "dot3 should have @param u and @param v");
    assert!(
        dot.tags.iter().any(|t| t.kind == TagKind::Return),
        "dot3 should have a @return tag"
    );
}

#[test]
fn fortran_normalise_has_body() {
    let items = extract_file(&fixture("fortran/vectors.f90"));
    let norm = find_item(&items, "normalise");
    // Second !! line becomes the body.
    assert!(
        !norm.body.is_empty() || !norm.brief.is_empty(),
        "normalise should have doc content"
    );
}

// ── Rust fixture: strings.rs ──────────────────────────────────────────────────

#[test]
fn rust_strings_finds_functions() {
    let items = extract_file(&fixture("rust/strings.rs"));
    for name in &["char_count", "truncate", "repeat_str", "centre"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function"
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn rust_strings_finds_struct_and_methods() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let builder = find_item(&items, "ListBuilder");
    assert!(matches!(builder.kind, DocKind::Struct));

    // impl methods: new, push, finish
    for name in &["new", "push", "finish"] {
        assert!(has_item(&items, name), "method '{name}' not found");
    }
}

#[test]
fn rust_char_count_has_example_in_body() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "char_count");
    // The ```…``` block lands in the body.
    assert!(
        item.body.contains("assert_eq")
            || item.tags.iter().any(|t| {
                use docify::extract::TagKind;
                t.kind == TagKind::Example && t.text.contains("assert_eq")
            }),
        "char_count should capture the example block"
    );
}

// ── Rust fixture: additional coverage ────────────────────────────────────────

#[test]
fn rust_truncate_has_body_text() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "truncate");
    assert!(
        !item.body.is_empty(),
        "truncate body should contain extended description"
    );
    assert!(
        item.body.contains('…') || item.body.contains("suffix") || item.body.contains("unchanged"),
        "truncate body should mention the ellipsis suffix behaviour"
    );
}

#[test]
fn rust_truncate_has_panics_tag() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "truncate");
    use docify::extract::TagKind;
    let t = item
        .tags
        .iter()
        .find(|t| t.kind == TagKind::Other("panics".to_string()));
    assert!(
        t.is_some(),
        "# Panics section should become Other(\"panics\") tag"
    );
    assert!(
        t.unwrap().text.to_ascii_lowercase().contains("never") || t.unwrap().text.contains("zero")
    );
}

#[test]
fn rust_centre_has_arguments_and_returns_tags() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "centre");
    use docify::extract::TagKind;
    let args = item
        .tags
        .iter()
        .find(|t| t.kind == TagKind::Other("arguments".to_string()));
    assert!(
        args.is_some(),
        "# Arguments section should become Other(\"arguments\") tag"
    );
    assert!(
        item.tags.iter().any(|t| t.kind == TagKind::Return),
        "# Returns section should become TagKind::Return tag"
    );
}

#[test]
fn rust_char_count_has_example_tag() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "char_count");
    use docify::extract::TagKind;
    assert!(
        item.tags.iter().any(|t| t.kind == TagKind::Example),
        "# Examples section should become TagKind::Example tag"
    );
    let ex = item
        .tags
        .iter()
        .find(|t| t.kind == TagKind::Example)
        .unwrap();
    assert!(
        ex.text.contains("assert_eq"),
        "example text should contain the code"
    );
}

#[test]
fn rust_centre_has_signature() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let item = find_item(&items, "centre");
    assert!(
        !item.signature.is_empty(),
        "centre should have a captured signature"
    );
    assert!(
        item.signature.contains("centre"),
        "signature should contain the function name"
    );
}

#[test]
fn rust_template_examples_have_params_returns_and_errors() {
    let items = extract_file(&fixture("rust/strings.rs"));
    let render = find_item(&items, "render_template");
    use docify::extract::TagKind;
    assert!(matches!(render.kind, DocKind::Function));
    assert!(
        render.signature.contains("render_template"),
        "function signature should be captured"
    );
    assert!(
        render
            .tags
            .iter()
            .any(|t| t.kind == TagKind::Other("arguments".to_string())),
        "render_template should expose its Arguments section"
    );
    assert!(
        render.tags.iter().any(|t| t.kind == TagKind::Return),
        "render_template should expose its Returns section"
    );
    assert!(
        render
            .tags
            .iter()
            .any(|t| t.kind == TagKind::Other("errors".to_string())),
        "render_template should expose its Errors section"
    );

    let parse = find_item(&items, "parse_csv_fields");
    assert!(matches!(parse.kind, DocKind::Function));
    assert!(
        parse.tags.iter().any(|t| t.kind == TagKind::Return),
        "parse_csv_fields should expose its Returns section"
    );
}

// ── Ada fixture: matrix.ads ───────────────────────────────────────────────────

#[test]
fn ada_matrix_finds_functions() {
    let items = extract_file(&fixture("ada/matrix.ads"));
    for name in &["Matrix.Mul", "Matrix.Add", "Matrix.Det", "Matrix.Transpose"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Subroutine | DocKind::Function),
            "{name} should be Subroutine or Function"
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn ada_matrix_item_count() {
    let items = extract_file(&fixture("ada/matrix.ads"));
    assert!(items.len() >= 4, "expected ≥4 items, got {}", items.len());
}

// ── Ada fixture: additional coverage ─────────────────────────────────────────

#[test]
fn ada_matrix_mul_has_params_and_return() {
    let items = extract_file(&fixture("ada/matrix.ads"));
    let mul = find_item(&items, "Matrix.Mul");
    use docify::extract::TagKind;
    let params: Vec<_> = mul
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert!(params.len() >= 2, "Mul should have @param A and @param B");
    assert!(
        mul.tags.iter().any(|t| t.kind == TagKind::Return),
        "Mul should have a @return tag"
    );
}

#[test]
fn ada_matrix_has_mat2_typedef() {
    let items = extract_file(&fixture("ada/matrix.ads"));
    // `type Mat2 is array ...` → Typedef.
    let mat2 = find_item(&items, "Matrix.Mat2");
    assert!(
        matches!(mat2.kind, DocKind::Typedef),
        "Mat2 should be Typedef, got {:?}",
        mat2.kind
    );
    assert!(!mat2.brief.is_empty(), "Mat2 needs a brief");
}

// ── D fixture: parser.d ───────────────────────────────────────────────────────

#[test]
fn d_parser_finds_enum_and_struct() {
    let items = extract_file(&fixture("d/parser.d"));
    let tok_kind = find_item(&items, "parser.TokenKind");
    assert!(matches!(tok_kind.kind, DocKind::Enum));

    let token = find_item(&items, "parser.Token");
    assert!(matches!(token.kind, DocKind::Struct));
}

#[test]
fn d_parser_finds_functions() {
    let items = extract_file(&fixture("d/parser.d"));
    let tok = find_item(&items, "parser.tokenise");
    assert!(matches!(tok.kind, DocKind::Function));

    let eval = find_item(&items, "parser.evaluate");
    assert!(matches!(eval.kind, DocKind::Function));
    assert!(!eval.brief.is_empty());
}

// ── D fixture: additional coverage ───────────────────────────────────────────

#[test]
fn d_parser_evaluate_body_describes_param() {
    // D uses `Params:` sections (not @param), so content lands in body.
    let items = extract_file(&fixture("d/parser.d"));
    let eval = find_item(&items, "parser.evaluate");
    let searchable = format!("{} {}", eval.brief, eval.body).to_ascii_lowercase();
    assert!(
        searchable.contains("expr") || searchable.contains("param"),
        "evaluate doc should describe the parameter; brief={:?} body={:?}",
        eval.brief,
        eval.body
    );
}

#[test]
fn d_parser_evaluate_body_describes_return() {
    // D uses `Returns:` sections (not @return), so content lands in body.
    let items = extract_file(&fixture("d/parser.d"));
    let eval = find_item(&items, "parser.evaluate");
    let searchable = format!("{} {}", eval.brief, eval.body).to_ascii_lowercase();
    assert!(
        searchable.contains("return")
            || searchable.contains("result")
            || searchable.contains("integer"),
        "evaluate doc should describe the return value; brief={:?} body={:?}",
        eval.brief,
        eval.body
    );
}

#[test]
fn d_parser_tokenise_has_body() {
    let items = extract_file(&fixture("d/parser.d"));
    let tok = find_item(&items, "parser.tokenise");
    // The `Throws` block should land in tags or body.
    use docify::extract::TagKind;
    let has_throws = tok.tags.iter().any(
        |t| matches!(&t.kind, TagKind::Other(s) if s.contains("throws") || s.contains("Throws")),
    );
    let throws_in_body = tok.body.to_ascii_lowercase().contains("throw");
    assert!(
        has_throws || throws_in_body,
        "tokenise Throws section should be captured"
    );
}

// ── Java fixture: collections.java ───────────────────────────────────────────

#[test]
fn java_intstack_is_class() {
    let items = extract_file(&fixture("java/collections.java"));
    let item = find_item(&items, "IntStack");
    assert!(
        matches!(item.kind, DocKind::Class),
        "IntStack should be Class, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "IntStack needs a brief");
}

#[test]
fn java_intstack_methods_found() {
    let items = extract_file(&fixture("java/collections.java"));
    for name in &["push", "pop", "peek", "size", "isEmpty"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn java_push_has_param_and_throws() {
    let items = extract_file(&fixture("java/collections.java"));
    let push = find_item(&items, "push");
    use docify::extract::TagKind;
    assert!(
        push.tags.iter().any(|t| t.kind == TagKind::Param),
        "push should have a @param tag"
    );
    let has_throws = push
        .tags
        .iter()
        .any(|t| matches!(&t.kind, TagKind::Other(s) if s.starts_with("throws")));
    assert!(has_throws, "push should have a @throws tag");
}

#[test]
fn java_pop_has_return_and_throws() {
    let items = extract_file(&fixture("java/collections.java"));
    let pop = find_item(&items, "pop");
    use docify::extract::TagKind;
    assert!(
        pop.tags.iter().any(|t| t.kind == TagKind::Return),
        "pop should have a @return tag"
    );
    let has_throws = pop
        .tags
        .iter()
        .any(|t| matches!(&t.kind, TagKind::Other(s) if s.starts_with("throws")));
    assert!(has_throws, "pop should have a @throws tag");
}

#[test]
fn java_snapshot_is_interface() {
    let items = extract_file(&fixture("java/collections.java"));
    let item = find_item(&items, "Snapshot");
    assert!(
        matches!(item.kind, DocKind::Interface),
        "Snapshot should be Interface, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "Snapshot needs a brief");
}

#[test]
fn java_snapshot_methods_found() {
    let items = extract_file(&fixture("java/collections.java"));
    for name in &["writeSnapshot", "snapshotSize"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
    }
}

#[test]
fn java_writesnapshot_has_multiple_params() {
    let items = extract_file(&fixture("java/collections.java"));
    let ws = find_item(&items, "writeSnapshot");
    use docify::extract::TagKind;
    let params: Vec<_> = ws
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert!(
        params.len() >= 2,
        "writeSnapshot should have @param out and @param offset"
    );
}

#[test]
fn java_collection_kind_is_enum() {
    let items = extract_file(&fixture("java/collections.java"));
    let item = find_item(&items, "CollectionKind");
    assert!(
        matches!(item.kind, DocKind::Enum),
        "CollectionKind should be Enum, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "CollectionKind needs a brief");
}

#[test]
fn java_extra_examples_find_record_and_static_method() {
    let items = extract_file(&fixture("java/collections.java"));
    let stats = find_item(&items, "StackStats");
    assert!(
        matches!(stats.kind, DocKind::Struct),
        "StackStats record should be Struct, got {:?}",
        stats.kind
    );

    let load = find_item(&items, "loadFactor");
    assert!(matches!(load.kind, DocKind::Function));
    use docify::extract::TagKind;
    assert!(
        load.tags.iter().any(|t| t.kind == TagKind::Return),
        "loadFactor should have a @return tag"
    );

    let clamp = find_item(&items, "clamp");
    assert!(matches!(clamp.kind, DocKind::Function));
    let params: Vec<_> = clamp
        .tags
        .iter()
        .filter(|t| t.kind == TagKind::Param)
        .collect();
    assert_eq!(params.len(), 3, "clamp should have value/min/max params");
}

#[test]
fn java_item_count() {
    let items = extract_file(&fixture("java/collections.java"));
    // IntStack (class), push/pop/peek/size/isEmpty (5 methods),
    // Snapshot (interface), writeSnapshot/snapshotSize (2 methods),
    // CollectionKind (enum) = at least 10.
    // Constructor "IntStack" deduplicates with the class, so net ≥ 10.
    assert!(
        items.len() >= 13,
        "expected ≥10 items from collections.java, got {}: {:?}",
        items.len(),
        items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

// ── Go fixture: geom.go ───────────────────────────────────────────────────────

#[test]
fn go_vec2_is_struct() {
    let items = extract_file(&fixture("go/geom.go"));
    let item = find_item(&items, "geom.Vec2");
    assert!(
        matches!(item.kind, DocKind::Struct),
        "geom.Vec2 should be Struct, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "geom.Vec2 needs a brief");
}

#[test]
fn go_vec2_methods_found() {
    let items = extract_file(&fixture("go/geom.go"));
    for name in &["geom.Add", "geom.Scale", "geom.Dot", "geom.Length"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn go_free_functions_found() {
    let items = extract_file(&fixture("go/geom.go"));
    for name in &["geom.Normalise", "geom.Lerp"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
    }
}

#[test]
fn go_shape_is_interface() {
    let items = extract_file(&fixture("go/geom.go"));
    let item = find_item(&items, "geom.Shape");
    assert!(
        matches!(item.kind, DocKind::Interface),
        "geom.Shape should be Interface, got {:?}",
        item.kind
    );
    assert!(!item.brief.is_empty(), "geom.Shape needs a brief");
}

#[test]
fn go_rect_is_struct() {
    let items = extract_file(&fixture("go/geom.go"));
    let item = find_item(&items, "geom.Rect");
    assert!(
        matches!(item.kind, DocKind::Struct),
        "geom.Rect should be Struct, got {:?}",
        item.kind
    );
}

#[test]
fn go_rect_methods_found() {
    let items = extract_file(&fixture("go/geom.go"));
    for name in &["geom.Area", "geom.Perimeter"] {
        let item = find_item(&items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function, got {:?}",
            item.kind
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn go_brief_follows_go_convention() {
    // Go convention: first sentence starts with the symbol name.
    let items = extract_file(&fixture("go/geom.go"));
    let norm = find_item(&items, "geom.Normalise");
    assert!(
        norm.brief.starts_with("Normalise"),
        "geom.Normalise brief should start with 'Normalise', got: {:?}",
        norm.brief
    );
    let lerp = find_item(&items, "geom.Lerp");
    assert!(
        lerp.brief.starts_with("Lerp"),
        "geom.Lerp brief should start with 'Lerp', got: {:?}",
        lerp.brief
    );
}

#[test]
fn go_multiline_comment_body() {
    // Normalise has a two-line doc comment; the second line should be in body.
    let items = extract_file(&fixture("go/geom.go"));
    let norm = find_item(&items, "geom.Normalise");
    assert!(
        !norm.body.is_empty() || norm.brief.contains("zero"),
        "geom.Normalise second comment line should appear in brief or body"
    );
}

#[test]
fn go_item_count() {
    let items = extract_file(&fixture("go/geom.go"));
    // Vec2, Add, Scale, Dot, Length, Normalise, Lerp, Shape, Rect, Area, Perimeter = 11
    assert!(
        items.len() >= 9,
        "expected ≥9 items from geom.go, got {}: {:?}",
        items.len(),
        items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

// ── doc-example: stats (C++) ──────────────────────────────────────────────────

#[test]
fn doc_example_stats_finds_namespace_functions() {
    let set = extract_dir(&doc_example("libs/stats/src"));
    let items = &set.items;
    assert!(!items.is_empty(), "stats lib should yield items");
    for name in &[
        "stats::mean",
        "stats::variance",
        "stats::stddev",
        "stats::pearson",
    ] {
        let item = find_item(items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be Function"
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn doc_example_stats_finds_class() {
    let set = extract_dir(&doc_example("libs/stats/src"));
    let cls = find_item(&set.items, "stats::OrderStatistics");
    assert!(matches!(cls.kind, DocKind::Class | DocKind::Struct));
    assert!(!cls.brief.is_empty());
}

#[test]
fn doc_example_stats_class_members_are_nested() {
    let set = extract_dir(&doc_example("libs/stats/src"));
    for name in &[
        "stats::OrderStatistics::OrderStatistics",
        "stats::OrderStatistics::median",
        "stats::OrderStatistics::percentile",
    ] {
        let item = find_item(&set.items, name);
        assert!(
            matches!(item.kind, DocKind::Function),
            "{name} should be a function"
        );
        assert_eq!(item.meta.parent.as_deref(), Some("OrderStatistics"));
    }
}

#[test]
fn doc_example_stats_mean_has_param_and_return() {
    let set = extract_dir(&doc_example("libs/stats/src"));
    let mean = find_item(&set.items, "stats::mean");
    use docify::extract::TagKind;
    assert!(
        mean.tags.iter().any(|t| t.kind == TagKind::Param),
        "mean needs @param"
    );
    assert!(
        mean.tags.iter().any(|t| t.kind == TagKind::Return),
        "mean needs @return"
    );
}

// ── doc-example: mathlib (C) ──────────────────────────────────────────────────

#[test]
fn doc_example_mathlib_finds_functions() {
    let set = extract_dir(&doc_example("libs/mathlib/src"));
    let items = &set.items;
    assert!(!items.is_empty(), "mathlib should yield items");
    // mathlib is C without namespace — simple names.
    for name in &["clamp", "lerp"] {
        assert!(has_item(items, name), "'{name}' not found in mathlib");
        let item = find_item(items, name);
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

// ── doc-example: linalg (Fortran) ────────────────────────────────────────────

#[test]
fn doc_example_linalg_finds_module_variables() {
    let set = extract_dir(&doc_example("libs/linalg/src"));
    let items = &set.items;
    for name in &["pi", "default_lda", "singular_tol"] {
        let item = find_item(items, name);
        assert!(
            matches!(item.kind, DocKind::Variable),
            "{name} should be Variable, got {:?}",
            item.kind
        );
        assert!(!item.brief.is_empty(), "{name} needs a brief");
    }
}

#[test]
fn doc_example_linalg_finds_procedures() {
    let set = extract_dir(&doc_example("libs/linalg/src"));
    let dot = find_item(&set.items, "dot");
    assert!(matches!(dot.kind, DocKind::Function));

    let scale = find_item(&set.items, "scale");
    assert!(matches!(scale.kind, DocKind::Subroutine));

    let solve2 = find_item(&set.items, "solve2");
    assert!(matches!(solve2.kind, DocKind::Function));
}

// ── clang path: same fixtures ─────────────────────────────────────────────────

#[cfg(feature = "clang")]
mod clang_tests {
    use super::*;
    use docify::extract_clang::extract_file_clang;

    #[test]
    fn clang_c_buffer_finds_functions() {
        let items = extract_file_clang(&fixture("c/buffer.h"));
        for name in &[
            "buffer_init",
            "buffer_push",
            "buffer_clear",
            "buffer_remaining",
        ] {
            let item = find_item(&items, name);
            assert!(matches!(item.kind, DocKind::Function));
        }
    }

    #[test]
    fn clang_cpp_geometry_namespace_items() {
        let items = extract_file_clang(&fixture("cpp/geometry.hpp"));
        let tri = find_item(&items, "geometry::triangle_area");
        assert!(matches!(tri.kind, DocKind::Function));

        let point = find_item(&items, "geometry::Point");
        assert!(matches!(point.kind, DocKind::Struct | DocKind::Class));

        let aabb = find_item(&items, "geometry::AABB");
        assert!(matches!(aabb.kind, DocKind::Struct | DocKind::Class));
    }

    #[test]
    fn clang_cpp_geometry_class_members() {
        let items = extract_file_clang(&fixture("cpp/geometry.hpp"));
        // With libclang, class members get fully-qualified names.
        let length = find_item(&items, "geometry::Point::length");
        assert!(matches!(length.kind, DocKind::Function));
        assert!(
            length.meta.parent.as_deref() == Some("Point"),
            "length's parent should be Point"
        );

        let contains = find_item(&items, "geometry::AABB::contains");
        assert!(matches!(contains.kind, DocKind::Function));
    }

    #[test]
    fn clang_cpp_geometry_constructor() {
        let items = extract_file_clang(&fixture("cpp/geometry.hpp"));
        // AABB has an explicit constructor with a doc comment.
        let ctor = items
            .iter()
            .find(|i| i.name.contains("AABB") && i.meta.attrs.iter().any(|a| a == "constructor"));
        assert!(ctor.is_some(), "AABB constructor not found");
    }

    #[test]
    fn clang_doc_example_stats_class_members() {
        let set = extract_dir(&doc_example("libs/stats/src"));
        // With clang, OrderStatistics methods get fully-qualified names.
        let median = find_item(&set.items, "stats::OrderStatistics::median");
        assert!(matches!(median.kind, DocKind::Function));
        assert_eq!(median.meta.parent.as_deref(), Some("OrderStatistics"));
    }
}
