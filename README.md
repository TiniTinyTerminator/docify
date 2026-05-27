# docify

Multi-language doc comment extractor and Markdown renderer. Reads doc comments from source files and writes structured GFM documentation — or serves them over a terminal browser.

![docify browse TUI, search, source, and gen commands](https://raw.githubusercontent.com/TiniTinyTerminator/freight-workspace/main/tapes/freight-doc-tui.gif)

## Supported languages

| Language | Comment styles | Tag format |
|---|---|---|
| C / C++ | `/** */`, `/*! */`, `///` | Doxygen `@param` / `@return` / `@brief` / … |
| Fortran | `!>` opener, `!!` continuation | FORD conventions |
| Rust | `///`, `/** */` | Markdown prose sections |
| D | `/++ +/`, `/** */`, `///` | DDoc |
| Ada | `--!`, `---` | Prose |
| Java | `/** */` | Javadoc `@param` / `@return` / `@throws` |
| Go | `//` block immediately before declaration | Prose |

## CLI

```sh
# Scan current directory, write Markdown to target/doc/
docify

# Scan specific directories
docify gen src/ include/

# Preview extracted items without writing files
docify gen --dry-run

# Custom output directory
docify gen --out docs/api

# Look up a symbol (JSON)
docify get stats::mean src/

# Print source code for a symbol
docify source OrderStatistics src/

# Symbol + doc in one response (JSON, useful for AI agents)
docify context stats::mean src/

# Search by name or description (JSON array)
docify search "least squares" src/

# Print all documented symbols as a compact JSON outline
docify outline src/

# Browse documentation interactively
docify browse src/
```

## Library

```toml
[dependencies]
docify = "0.1"
```

```rust
use docify::extract::{extract_dir, DocSet};
use docify::render;

let set: DocSet = extract_dir(std::path::Path::new("src/"));
render(&set, std::path::Path::new("target/doc/"))?;
```

## Namespace and package qualification

Extracted items are qualified with their enclosing scope:

- **C++** — `stats::mean`, `stats::algo::sort_n`
- **Rust** — `geometry::Vec2::normalise`
- **Go** — `geom.Add`
- **Ada** — `Matrix.Mul`
- **D** — `parser.tokenise`

C++ Doxygen groups (`@defgroup` / `@addtogroup` / `@{` / `@}`) are also tracked; items belong to both their group and their namespace.

## Optional libclang backend

For accurate C/C++ member extraction (class members, template parameters, access specifiers), build with the `clang` feature:

```toml
docify = { version = "0.1", features = ["clang"] }
```

Requires `libclang` on the system. Falls back to the heuristic extractor automatically if unavailable.

## Math support

LaTeX math in doc bodies — `$...$`, `$$...$$`, `\(...\)`, `\[...\]` — is preserved verbatim in Markdown and JSON output. The TUI browser (`docify browse`) converts it to Unicode symbols (Greek letters, ∑ ∫ √ ≤ ≥ ×, super/subscripts).

## Output structure

`docify gen` writes:

```
target/doc/
  index.md          — overview + namespace/module listing
  symbols.md        — alphabetical symbol index
  namespace/        — one page per C++ namespace or Rust mod
  class/            — one page per class or struct
  module/           — one page per Fortran MODULE
  group/            — one page per Doxygen @defgroup group
```

## License

Apache-2.0
