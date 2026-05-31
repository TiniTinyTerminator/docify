# multi-project

A workspace containing six independent sub-projects, each using a
different package manager.  Used to test and demonstrate docify's
project-boundary detection and per-project grouping.

## Sub-projects

| Directory | Package manager | Manifest | Language |
|-----------|----------------|----------|----------|
| `web-utils/` | **npm** | `package.json` | TypeScript |
| `string-proc/` | **Cargo** | `Cargo.toml` | Rust |
| `native-math/` | **freight** | `freight.toml` | C++ |
| `analysis/` | **Hatch / pip** | `pyproject.toml` | Python |
| `http-kit/` | **Go modules** | `go.mod` | Go |
| `phputils/` | **Composer** | `composer.json` | PHP |

## Browsing

```sh
# From the docify crate root:
cargo run -- browse examples/multi-project
```

Without project-boundary detection docify shows all ~50 symbols flat
(grouped only by language).  With project detection each sub-project
becomes a top-level node in the tree:

```
web-utils  1.2.0
  TypeScript
    Classes & Types — BoundingBox
    Free Symbols    — parseQuery · parseQueryAll · buildQuery · …
string-proc  0.3.1
  Rust
    Free Symbols    — to_snake · to_camel · slugify · …
native-math  0.5.0
  C++
    Namespaces      — nm
      …
…
```
