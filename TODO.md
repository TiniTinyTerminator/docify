# docify TODO

## Language support

Currently extracted: C/C++, Rust, Fortran, D, Ada, Java, Go.
Languages freight supports that docify does not yet extract:

| Language   | Comment style           | Notes                              |
|------------|-------------------------|------------------------------------|
| ~~**Zig**~~    | ~~`///` doc comments~~  | Done in `src/extract/zig.rs`       |
| ~~**Swift**~~  | ~~`///` and `/** */`~~  | Done in `src/extract/swift.rs`     |
| ~~**Kotlin**~~ | ~~`/** */` KDoc~~       | Done in `src/extract/kotlin.rs`    |
| **CUDA**   | Doxygen `/** */`        | Falls through to Cpp; `__global__` / `__device__` qualifier display handled; still needs broader CUDA semantic coverage |
| **ISPC**   | C-style `/** */`        | `task`, `export`, `uniform` qualifiers |
| **HIP**    | Doxygen `/** */`        | Same as CUDA, different builtins   |
| **Python** | Docstrings `""" """`    | Requires different extraction logic (not comment-based) |

## Output formats

Current outputs: Markdown pages (per-namespace, per-file, index), JSON symbol list,
and a terminal TUI (`render_tui.rs`). Gaps:

- **HTML**: Single-page or multi-page static site output. The Markdown renderer
  is already structured; an HTML backend could reuse most of the same grouping logic.
- **MessagePack**: Wire format is used in the `freight doc` agent protocol (`agent.rs`).
  Ensure the MessagePack schema is documented and versioned.

## Extraction quality

- **Language-specific symbol kinds**: `DocKind` is currently a single cross-language
  enum. That is too coarse for languages with different type systems and symbol
  categories (Python modules/classes/functions/properties, Rust traits/impls/macros,
  C++ concepts/templates/operators, etc.). Consider replacing or supplementing it
  with a language-specific enum and mapping those variants to coarse categories
  only at render/API boundaries.
- **C/C++ `@param` direction tags**: `[in]`, `[out]`, `[in,out]` in Doxygen params
  are parsed but not stored in `DocTag`. Add a `direction: Option<ParamDir>` field.
- **Rust intra-doc links**: `[`SomeType`]` references are preserved verbatim in the
  Markdown output. Resolve them to anchor links when the target is in the same doc set.
- **Multi-file cross-references**: The index groups by file but does not link
  `@see` / `\see` / `see also` tags to their target symbols across files.
- **Ada**: `--!` doc comments are extracted, but `procedure`/`function` signature
  cleanup (`sig_ada`) does not handle generic formal parameters. Track as a known
  gap until a real Ada project is tested.

## Testing

- Tests for the Go extractor's package-qualification logic (multi-package repos).
- Tests for Java inner classes and interface methods.
- Test round-trip: extract → JSON → re-parse and compare field by field.
- ~~Test that CUDA `__global__` functions are correctly classified as `DocKind::Function`.~~
