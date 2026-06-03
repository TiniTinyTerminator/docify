# doc-example

A small multi-language project demonstrating **freight-doc** extraction and the
TUI documentation browser.

## Libraries

| Dep | Language | Doc style | Purpose |
|-----|----------|-----------|---------|
| `mathlib` | C17 | Doxygen `/** */` | Numerical methods (bisection, quadrature, LU) |
| `stats` | C++17 | Doxygen `/** */` / `///` | Descriptive statistics and regression |
| `shapes` | C++17 | Doxygen `/** */` | Virtual base class, overloaded constructors and functions |
| `linalg` | Fortran 2018 | FORD `!>` / `!!` | Dense linear algebra (dev-dependency) |
| `signals` | Python + R | Docstrings / Roxygen2 `#'` | Signal processing utilities |
| `geometry` | TypeScript | JSDoc `/** */` | 2-D / 3-D geometry primitives |
| `csvkit` | C# | XML doc `///` | CSV reader / writer |
| `formatter` | Ruby | YARD `# ` blocks | Text formatting and table rendering |
| `scripting` | Lua + Haskell | LuaDoc `---` / Haddock `-- \|` | Vector math, pure statistics |

## Rendered doc features

This example exercises every doc-comment feature across 10 languages:

- **Doxygen** `@brief` / `@param` / `@return` / `@see` / `@warning` tags (C, C++) — including virtual base classes, constructor overloads, and free-function overloads (`shapes`)
- **FORD** `!>` and `!!` Fortran inline comments
- **Python** Google-style docstrings with `Args:` / `Returns:` / `Raises:` sections
- **JSDoc** `@param` / `@returns` / `@see` (TypeScript)
- **XML doc** `<summary>` / `<param>` / `<returns>` / `<exception>` (C#)
- **YARD** `@param [Type]` / `@return` / `@see` (Ruby)
- **LuaDoc** `---` blocks with `@param` / `@return`, EmmyLua `---@` style (Lua)
- **Roxygen2** `#'` blocks with `@param` / `@return` / `@seealso` (R)
- **Haddock** `-- |` and `{- | -}` blocks (Haskell)
- Inline math: $f(x) = e^{-x^2}$
- Display math: $$\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}$$
- Markdown tables *inside* doc-comment bodies
- Cross-reference links (`@see bisect` → navigable link to `bisect`)

## Running

```sh
cd examples/doc-example
freight build
freight doc
```

`freight doc` opens the interactive TUI browser. Press **Enter** on a package
to expand it and load its README and API tree. Use **Tab** to switch focus
between the tree, content, and info panels. Press **q** to quit.

## Key bindings

| Key | Action |
|-----|--------|
| `↑` / `↓` or `k` / `j` | Navigate tree / scroll content |
| `Enter` | Expand dep or open symbol |
| `Tab` | Cycle focus: tree → content → info |
| `Esc` / `Backspace` | Return focus to tree |
| `g` / `G` | Jump to top / bottom |
| `PgUp` / `PgDn` | Page up / down |
| `q` | Quit |
