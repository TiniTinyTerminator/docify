# freight-doc

Multi-language doc comment extractor and Markdown renderer. Ships as both a library (`freight_doc`) and a standalone CLI (`freight-doc`).

## Supported languages

| Language | Comment styles | Tag format |
|----------|---------------|------------|
| C / C++ | `/** */`, `/*! */`, `///` | Doxygen `@param` / `@return` / `@brief` |
| Fortran | `!>` opener, `!!` continuation | FORD conventions |
| Rust | `///`, `/** */` | Markdown prose |
| D | `/++ +/`, `/** */`, `///` | DDoc |
| Ada | `--!`, `---` | Prose |
| Java | `/** */` | Javadoc `@param` / `@return` / `@throws` |
| Go | `//` block immediately before declaration | Prose |

Optionally use libclang for accurate C/C++ AST extraction (member functions, templates, access specifiers):

```toml
freight-doc = { version = "0.1", features = ["clang"] }
```

## CLI usage

```sh
# Scan current directory, write Markdown to target/doc/
freight-doc

# Scan specific directories
freight-doc src/ include/

# Preview extracted items without writing files
freight-doc --dry-run

# Custom output directory
freight-doc --out docs/api
```

## Library usage

```rust
use freight_doc::extract::{extract_dir, DocSet};
use freight_doc::render;

let set: DocSet = extract_dir(std::path::Path::new("src/"));
render(&set, std::path::Path::new("target/doc/"))?;
```

### Custom language extractors

Implement `DocExtractor` to add support for languages not built in:

```rust
use freight_doc::extract::{DocExtractor, DocItem, extract_dir_with};

struct PythonExtractor;

impl DocExtractor for PythonExtractor {
    fn extensions(&self) -> &[&str] { &["py"] }
    fn extract(&self, path: &std::path::Path, source: &str) -> Vec<DocItem> {
        // parse triple-quote docstrings …
        vec![]
    }
}

let extras: Vec<Box<dyn DocExtractor>> = vec![Box::new(PythonExtractor)];
let set = extract_dir_with(std::path::Path::new("src/"), &extras);
```

## Module overview

| Path | Responsibility |
|------|---------------|
| `src/extract/mod.rs` | `DocExtractor` trait, entry points, language dispatch |
| `src/extract/common.rs` | Shared helpers: `build_item`, tag parsing, block collectors |
| `src/extract/{cpp,rust,fortran,d,ada,java,go}.rs` | Per-language extractors |
| `src/extract_clang.rs` | libclang AST walker (`clang` feature) |
| `src/render_md.rs` | GFM Markdown output with per-namespace / per-class pages |
| `src/markdown.rs` | Math protection (`$…$`, `$$…$$`) and Markdown utilities |
| `src/main.rs` | `freight-doc` CLI |

## License

Apache-2.0
