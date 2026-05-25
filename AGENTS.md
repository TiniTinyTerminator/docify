# AGENTS.md — docify

Extracts structured doc comments from source files in multiple languages and renders
them as Markdown pages, JSON, MessagePack, or a terminal TUI. Used by `freight doc`.

See `TODO.md` for open work.

---

## Architecture

```
src/
├── lib.rs              — public API: extract_file(), extract_dir(), DocSet, DocItem, …
├── main.rs             — CLI binary (docify)
├── agent.rs            — freight doc wire protocol (JSON/MessagePack over stdout)
├── markdown.rs         — LaTeX math placeholder handling for Markdown output
├── render_md.rs        — Markdown page renderer (per-namespace, per-file, index)
├── render_tui.rs       — terminal output renderer
├── tui.rs              — interactive terminal UI
└── extract/
    ├── mod.rs          — DocExtractor trait, DocItem, DocSet, ExtractorRegistry
    ├── common.rs       — shared helpers: collect_c_block, collect_line_block, build_item, …
    ├── cpp.rs          — C / C++ (Doxygen /** */ and ///)
    ├── rust.rs         — Rust (/// and /** */)
    ├── fortran.rs      — Fortran (!> and !!)
    ├── ada.rs          — Ada (--!)
    ├── d.rs            — D (/++ +/)
    ├── java.rs         — Java (/** */ Javadoc)
    └── go.rs           — Go (// godoc)
```

---

## Core types (`extract/mod.rs`)

```rust
pub trait DocExtractor: Send + Sync {
    fn extensions(&self) -> &[&str];
    fn extract(&self, path: &Path, source: &str) -> Vec<DocItem>;
}

pub struct DocItem {
    pub name:      String,       // qualified symbol name
    pub kind:      DocKind,      // Function | Type | Constant | Module | …
    pub lang:      DocLanguage,
    pub file:      PathBuf,
    pub line:      usize,        // 1-based
    pub brief:     String,       // first paragraph
    pub tags:      Vec<DocTag>,  // @param, @return, @throws, @note, …
    pub signature: String,       // raw declaration line
    pub access:    Access,       // Public | Protected | Private
    pub meta:      DocMeta,      // deprecated, since, group
}

pub struct DocSet {
    pub items: Vec<DocItem>,
}
```

`DocKind` variants: `Function`, `Type`, `Constant`, `Module`, `Namespace`,
`Subroutine`, `Variable`, `Unknown`.

`TagKind` variants: `Param`, `Return`, `Throws`, `Note`, `Example`, `See`,
`Since`, `Deprecated`, `Author`.

---

## Adding a new language extractor

1. **Add a `DocLanguage` variant** in `extract/mod.rs`:
   ```rust
   pub enum DocLanguage { C, Cpp, Rust, …, Zig, /* new */ }
   ```
   Add a `label()` arm and a `display_signature()` arm for the new variant.

2. **Add file extensions** to `lang_from_ext` in `extract/mod.rs`:
   ```rust
   "zig" => DocLanguage::Zig,
   ```

3. **Create `extract/zig.rs`** implementing `DocExtractor`:
   ```rust
   pub struct ZigExtractor;
   impl DocExtractor for ZigExtractor {
       fn extensions(&self) -> &[&str] { &["zig"] }
       fn extract(&self, path: &Path, src: &str) -> Vec<DocItem> {
           extract_zig(src, path)
       }
   }
   ```
   Use helpers from `extract/common.rs` — see `rust.rs` for a `///`-style template
   or `java.rs` for a `/** */`-style template.

4. **Register the extractor** in `extract/mod.rs` inside `ExtractorRegistry::new()`:
   ```rust
   reg.register(Box::new(zig::ZigExtractor));
   ```

5. **Add tests** in the new file. Each test should call `extract_zig(src, path)`
   directly (not through the registry) and assert on `DocItem` fields.

**Common extractor helpers in `common.rs`:**

| Helper | Use |
|---|---|
| `collect_line_block(lines, i, prefix)` | Gather consecutive `///`-style lines |
| `collect_c_block(lines, i)` | Gather a `/** … */` block |
| `build_item(block, name, kind, file, line, lang, sig)` | Construct a `DocItem` from a raw comment block |
| `item_has_content(item)` | Return false if neither brief nor tags were found |
| `next_non_blank(lines, i)` | Find the first non-empty line at or after `i` |
| `first_ident(s)` | Extract the first identifier from a string |
| `next_decl_sym(lines, i)` | Find the next declaration line (skips attributes/annotations) |

---

## Adding a new output format

Output formats are independent of extraction. The pipeline is:

```
DocSet  →  renderer  →  files / stdout
```

Existing renderers: `render_md.rs` (Markdown), `render_tui.rs` (terminal),
`agent.rs` (JSON/MessagePack for `freight doc`).

To add HTML:

1. Create `src/render_html.rs` with a function `pub fn render_html(set: &DocSet, out_dir: &Path) -> std::io::Result<()>`.
2. Mirror the grouping logic from `render_md.rs` (`Groups`, `by_namespace`, `by_file`).
3. Wire it into `main.rs` behind a `--format html` flag.
4. Wire it into `lib.rs::render()` if it should be callable from `freight doc`.

---

## Wire protocol (`agent.rs`)

`freight doc` runs `docify` as a subprocess and reads its stdout. The protocol:

- **JSON mode**: one JSON object per line, each matching `SymbolJson`.
- **MessagePack mode**: length-prefixed MessagePack frames, each a `SymbolJson` map.

`SymbolJson` is defined in `agent.rs`. If you change its fields, update the reader
in `freight/src/doc/` in the same commit. There is currently no schema version field
— adding one is tracked in `TODO.md`.

---

## Testing

Tests live in `#[cfg(test)]` blocks at the bottom of each extractor file.

Pattern for a new extractor test:

```rust
#[test]
fn extracts_function_brief() {
    let src = "/// Return the answer.\npub fn answer() -> u32 { 42 }";
    let path = std::path::Path::new("test.rs");
    let items = extract_rust(src, path);   // call the lang-specific fn directly
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "answer");
    assert_eq!(items[0].brief, "Return the answer.");
    assert!(matches!(items[0].kind, DocKind::Function));
}
```

Call the language-specific `extract_*` function directly — not `extract_file` or
the registry — so tests are fast and isolated from file I/O.

---

## What not to change

- `DocItem` field names and `DocTag`/`TagKind` variants are part of the `agent.rs`
  wire format. Renaming or removing a field requires a matching change in the
  `freight doc` reader.
- `common.rs` helpers are used by every extractor. Changes there can break multiple
  languages at once — test all affected extractors when modifying shared helpers.
- Do not add language-specific logic to `common.rs`; keep it in the per-language file.
